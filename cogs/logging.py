"""
Logging Cog
Logs to MOD_LOG_CHANNEL:
  - Member join / leave
  - Message edit / delete
  - Member ban / unban
"""
import discord
from discord.ext import commands
from utils.utils import log_action, utcnow

class Logging(commands.Cog):
    def __init__(self, bot):
        self.bot = bot

    @commands.Cog.listener()
    async def on_member_join(self, member: discord.Member):
        embed = discord.Embed(
            title="📥 Member Joined",
            color=discord.Color.green(),
            timestamp=utcnow()
        )
        embed.set_thumbnail(url=member.display_avatar.url)
        embed.add_field(name="User", value=f"{member} (`{member.id}`)")
        embed.add_field(name="Account Created", value=discord.utils.format_dt(member.created_at, 'R'))
        embed.set_footer(text=f"Member count: {member.guild.member_count}")
        await log_action(member.guild, embed)

    @commands.Cog.listener()
    async def on_member_remove(self, member: discord.Member):
        embed = discord.Embed(
            title="📤 Member Left",
            color=discord.Color.greyple(),
            timestamp=utcnow()
        )
        embed.add_field(name="User", value=f"{member} (`{member.id}`)")
        roles = [r.mention for r in member.roles if r.name != "@everyone"]
        embed.add_field(name="Roles", value=", ".join(roles) or "None")
        await log_action(member.guild, embed)

    @commands.Cog.listener()
    async def on_message_edit(self, before: discord.Message, after: discord.Message):
        if not before.guild or before.author.bot:
            return
        if before.content == after.content:
            return
        embed = discord.Embed(
            title="✏️ Message Edited",
            color=discord.Color.blue(),
            timestamp=utcnow()
        )
        embed.add_field(name="Author", value=f"{before.author} (`{before.author.id}`)")
        embed.add_field(name="Channel", value=before.channel.mention)
        embed.add_field(name="Before", value=before.content[:1024] or "*empty*", inline=False)
        embed.add_field(name="After",  value=after.content[:1024] or "*empty*", inline=False)
        embed.add_field(name="Jump", value=f"[Link]({after.jump_url})")
        await log_action(before.guild, embed)

    @commands.Cog.listener()
    async def on_message_delete(self, message: discord.Message):
        if not message.guild or message.author.bot:
            return
        embed = discord.Embed(
            title="🗑️ Message Deleted",
            color=discord.Color.red(),
            timestamp=utcnow()
        )
        embed.add_field(name="Author",  value=f"{message.author} (`{message.author.id}`)")
        embed.add_field(name="Channel", value=message.channel.mention)
        embed.add_field(name="Content", value=message.content[:1024] or "*empty/attachment*", inline=False)
        await log_action(message.guild, embed)

    @commands.Cog.listener()
    async def on_member_ban(self, guild: discord.Guild, user: discord.User):
        embed = discord.Embed(title="🔨 Member Banned", color=discord.Color.dark_red(), timestamp=utcnow())
        embed.add_field(name="User", value=f"{user} (`{user.id}`)")
        await log_action(guild, embed)

    @commands.Cog.listener()
    async def on_member_unban(self, guild: discord.Guild, user: discord.User):
        embed = discord.Embed(title="✅ Member Unbanned", color=discord.Color.green(), timestamp=utcnow())
        embed.add_field(name="User", value=f"{user} (`{user.id}`)")
        await log_action(guild, embed)

async def setup(bot):
    await bot.add_cog(Logging(bot))