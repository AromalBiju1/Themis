"""
Autorole Cog
Assigns AUTOROLE_ID to every new member on join.
"""
import discord
from discord.ext import commands
import logging
from config.config import AUTOROLE_ID

log = logging.getLogger("modbot.autorole")

class Autorole(commands.Cog):
    def __init__(self, bot):
        self.bot = bot

    @commands.Cog.listener()
    async def on_member_join(self, member: discord.Member):
        if not AUTOROLE_ID:
            return
        role = member.guild.get_role(AUTOROLE_ID)
        if not role:
            log.warning(f"AUTOROLE_ID {AUTOROLE_ID} not found in guild")
            return
        try:
            await member.add_roles(role, reason="Autorole on join")
        except discord.Forbidden:
            log.error("Missing Manage Roles permission for autorole")

async def setup(bot):
    await bot.add_cog(Autorole(bot))