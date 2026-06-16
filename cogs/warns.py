"""
Warn System Cog
Persistent warns via SQLite. Auto-escalates:
  WARN_MUTE_AT  → timeout 1 hour
  WARN_KICK_AT  → kick
  WARN_BAN_AT   → ban
"""
import discord
from discord.ext import commands
import logging
from config.config import WARN_MUTE_AT, WARN_KICK_AT, WARN_BAN_AT
from utils.utils import is_mod, log_action, mod_embed, utcnow
from db.db import init_db

log = logging.getLogger("modbot.warns")

class Warns(commands.Cog):
    def __init__(self, bot):
        self.bot = bot

    @commands.Cog.listener()
    async def on_ready(self):
        await init_db()

    @commands.command(name="warn")
    async def warn(self, ctx, member: discord.Member, *, reason: str = "No reason provided"):
        if not is_mod(ctx.author):
            return await ctx.send("❌ No permission.", delete_after=5)
        if member.guild_permissions.administrator:
            return await ctx.send("❌ Cannot warn an admin.", delete_after=5)

        ts = utcnow().isoformat()
        await db.add_warn(ctx.guild.id, member.id, ctx.author.id, reason, ts)
        warns = await db.get_warns(ctx.guild.id, member.id)
        count = len(warns)

        await ctx.send(f"⚠️ {member.mention} warned. Total warns: **{count}**")
        embed = mod_embed(
            "⚠️ Member Warned",
            discord.Color.yellow(),
            user=f"{member} (`{member.id}`)",
            moderator=str(ctx.author),
            reason=reason,
            total_warns=str(count)
        )
        await log_action(ctx.guild, embed)

        # Escalation
        await self._escalate(ctx, member, count)

    async def _escalate(self, ctx, member: discord.Member, count: int):
        import datetime
        if count >= WARN_BAN_AT:
            try:
                await ctx.guild.ban(member, reason=f"Reached {WARN_BAN_AT} warns", delete_message_seconds=0)
                await ctx.send(f"🔨 {member.mention} banned after {WARN_BAN_AT} warns.")
            except discord.Forbidden:
                pass
        elif count >= WARN_KICK_AT:
            try:
                await member.kick(reason=f"Reached {WARN_KICK_AT} warns")
                await ctx.send(f"👢 {member.mention} kicked after {WARN_KICK_AT} warns.")
            except discord.Forbidden:
                pass
        elif count >= WARN_MUTE_AT:
            try:
                await member.timeout(datetime.timedelta(hours=1), reason=f"Reached {WARN_MUTE_AT} warns")
                await ctx.send(f"🔇 {member.mention} timed out (1hr) after {WARN_MUTE_AT} warns.")
            except discord.Forbidden:
                pass

    @commands.command(name="warns")
    async def list_warns(self, ctx, member: discord.Member):
        if not is_mod(ctx.author):
            return await ctx.send("❌ No permission.", delete_after=5)
        warns = await db.get_warns(ctx.guild.id, member.id)
        if not warns:
            return await ctx.send(f"✅ {member.mention} has no warns.")

        embed = discord.Embed(title=f"Warns for {member}", color=discord.Color.yellow())
        for w in warns:
            wid, mod_id, reason, ts = w
            embed.add_field(name=f"#{wid} — {ts[:10]}", value=f"Mod: <@{mod_id}>\nReason: {reason}", inline=False)
        await ctx.send(embed=embed)

    @commands.command(name="unwarn")
    async def unwarn(self, ctx, warn_id: int):
        if not is_mod(ctx.author):
            return await ctx.send("❌ No permission.", delete_after=5)
        await db.remove_warn(warn_id)
        await ctx.send(f"✅ Warn #{warn_id} removed.")

    @commands.command(name="clearwarns")
    async def clearwarns(self, ctx, member: discord.Member):
        if not is_mod(ctx.author):
            return await ctx.send("❌ No permission.", delete_after=5)
        await db.clear_warns(ctx.guild.id, member.id)
        await ctx.send(f"✅ All warns cleared for {member.mention}.")

async def setup(bot):
    await bot.add_cog(Warns(bot))