"""
Antispam Cog
- Message rate limiting (N messages per X seconds → mute)
- Anti-invite link (delete + warn)
- Anti-raid detection (N joins per X seconds → lockdown)
"""
import discord
from discord.ext import commands
from collections import defaultdict, deque
import asyncio
import re
import time
import logging
from config.config import (
    SPAM_MSG_LIMIT, SPAM_WINDOW_SECS,
    ANTI_INVITE,
    RAID_JOIN_LIMIT, RAID_WINDOW_SECS,
    MUTE_ROLE_ID
)
from utils.utils import is_immune, log_action, mod_embed

log = logging.getLogger("modbot.antispam")

INVITE_RE = re.compile(r"(discord\.gg|discord\.com/invite)/[a-zA-Z0-9]+", re.IGNORECASE)

class Antispam(commands.Cog):
    def __init__(self, bot):
        self.bot = bot
        # user_id → deque of timestamps
        self._msg_times: dict[int, deque] = defaultdict(deque)
        # timestamps of recent joins for raid detection
        self._join_times: deque = deque()
        self._raid_mode = False

    # ── Message spam ──────────────────────────────────────────────────────────
    @commands.Cog.listener()
    async def on_message(self, message: discord.Message):
        if not message.guild or message.author.bot:
            return
        if is_immune(message.author):
            return

        await self._check_spam(message)
        if ANTI_INVITE:
            await self._check_invite(message)

    async def _check_spam(self, message: discord.Message):
        uid  = message.author.id
        now  = time.monotonic()
        dq   = self._msg_times[uid]

        dq.append(now)
        # Drop entries outside the window
        while dq and now - dq[0] > SPAM_WINDOW_SECS:
            dq.popleft()

        if len(dq) >= SPAM_MSG_LIMIT:
            dq.clear()
            member = message.author
            guild  = message.guild
            log.info(f"Spam detected from {uid}")

            # Delete recent messages
            try:
                await message.channel.purge(limit=SPAM_MSG_LIMIT, check=lambda m: m.author.id == uid)
            except discord.Forbidden:
                pass

            # Mute via timeout (10 minutes)
            try:
                import datetime
                await member.timeout(
                    datetime.timedelta(minutes=10),
                    reason="Spam detected"
                )
            except discord.Forbidden:
                pass

            embed = mod_embed(
                "🚫 Spam Detected",
                discord.Color.orange(),
                user=f"<@{uid}> (`{uid}`)",
                action="Timeout 10 minutes",
                channel=message.channel.mention
            )
            await log_action(guild, embed)

    async def _check_invite(self, message: discord.Message):
        if INVITE_RE.search(message.content):
            try:
                await message.delete()
            except (discord.NotFound, discord.Forbidden):
                pass

            try:
                await message.channel.send(
                    f"{message.author.mention} Invite links are not allowed.",
                    delete_after=5
                )
            except discord.Forbidden:
                pass

            embed = mod_embed(
                "🔗 Invite Link Blocked",
                discord.Color.yellow(),
                user=f"<@{message.author.id}>",
                channel=message.channel.mention
            )
            await log_action(message.guild, embed)

    # ── Raid detection ────────────────────────────────────────────────────────
    @commands.Cog.listener()
    async def on_member_join(self, member: discord.Member):
        now = time.monotonic()
        self._join_times.append(now)

        # Drop old entries
        while self._join_times and now - self._join_times[0] > RAID_WINDOW_SECS:
            self._join_times.popleft()

        if len(self._join_times) >= RAID_JOIN_LIMIT and not self._raid_mode:
            await self._enable_raid_mode(member.guild)

    async def _enable_raid_mode(self, guild: discord.Guild):
        self._raid_mode = True
        log.warning(f"RAID MODE ENABLED in {guild.id}")

        # Set verification level to highest
        try:
            await guild.edit(verification_level=discord.VerificationLevel.highest)
        except discord.Forbidden:
            pass

        embed = mod_embed(
            "🚨 RAID MODE ACTIVATED",
            discord.Color.dark_red(),
            reason=f"{RAID_JOIN_LIMIT} joins in {RAID_WINDOW_SECS}s",
            action="Verification level set to HIGHEST",
            note="Use !raidoff to disable"
        )
        await log_action(guild, embed)

        # Auto-disable after 10 minutes
        await asyncio.sleep(600)
        await self._disable_raid_mode(guild)

    async def _disable_raid_mode(self, guild: discord.Guild):
        self._raid_mode = False
        try:
            await guild.edit(verification_level=discord.VerificationLevel.medium)
        except discord.Forbidden:
            pass
        log.info("Raid mode disabled")
        embed = mod_embed("✅ Raid Mode Disabled", discord.Color.green(), action="Verification restored to medium")
        await log_action(guild, embed)

    @commands.command(name="raidoff")
    async def raidoff(self, ctx):
        """Manually disable raid mode."""
        from utils import is_mod
        if not is_mod(ctx.author):
            return
        await self._disable_raid_mode(ctx.guild)
        await ctx.send("✅ Raid mode disabled.", delete_after=5)

async def setup(bot):
    await bot.add_cog(Antispam(bot))