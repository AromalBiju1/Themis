"""
Honeypot Cog
Any message in the honeypot channel → softban (ban + immediate unban)
Discord's delete_message_seconds=86400 wipes their last 24hr msgs server-wide.
"""
import discord
from discord.ext import commands
import asyncio
import logging
from config.config import HONEYPOT_CHANNEL
from utils.utils import is_immune, log_action, mod_embed

log = logging.getLogger("modbot.honeypot")
_processing: set[int] = set()

class Honeypot(commands.Cog):
    def __init__(self, bot):
        self.bot = bot

    @commands.Cog.listener()
    async def on_message(self, message: discord.Message):
        if not message.guild:
            return
        if message.channel.id != HONEYPOT_CHANNEL:
            return
        if message.author.id == self.bot.user.id:
            return
        await self._handle(message)

    async def _handle(self, message: discord.Message):
        member = message.author
        guild  = message.guild

        if is_immune(member):
            return
        if member.id in _processing:
            return
        _processing.add(member.id)

        try:
            try:
                await message.delete()
            except (discord.NotFound, discord.Forbidden):
                pass

            await guild.ban(
                member,
                reason="Honeypot triggered",
                delete_message_seconds=86400
            )
            log.info(f"Softban step 1: banned {member.id}")
            await asyncio.sleep(1)
            await guild.unban(member, reason="Honeypot softban complete")
            log.info(f"Softban complete: unbanned {member.id}")

            embed = mod_embed(
                "🍯 Honeypot Triggered",
                discord.Color.red(),
                user=f"<@{member.id}> (`{member.id}`)",
                action="Softban — 24hr messages wiped server-wide"
            )
            await log_action(guild, embed)

        except discord.Forbidden:
            log.error(f"Missing ban permission for {member.id}")
        except discord.HTTPException as e:
            log.error(f"HTTPException during softban: {e}")
        finally:
            _processing.discard(member.id)

async def setup(bot):
    await bot.add_cog(Honeypot(bot))