"""
Mod Commands Cog
!ban !kick !softban !mute !unmute !purge !slowmode
All commands check mod role before executing.
"""
import discord
from discord.ext import commands
import datetime
import logging
from utils import is_mod, log_action, mod_embed

log = logging.getLogger("modbot.modcmds")

def mod_only():
    async def predicate(ctx):
        if not is_mod(ctx.author):
            await ctx.send("❌ No permission.", delete_after=5)
            return False
        return True
    return commands.check(predicate)

class ModCmds(commands.Cog):
    def __init__(self, bot):
        self.bot = bot

    @commands.command()
    @mod_only()
    async def ban(self, ctx, member: discord.Member, *, reason: str = "No reason provided"):
        try:
            await ctx.guild.ban(member, reason=reason, delete_message_seconds=0)
            await ctx.send(f"🔨 Banned {member}.")
            await log_action(ctx.guild, mod_embed("🔨 Ban", discord.Color.red(),
                user=f"{member} (`{member.id}`)", moderator=str(ctx.author), reason=reason))
        except discord.Forbidden:
            await ctx.send("❌ Missing permissions.")

    @commands.command()
    @mod_only()
    async def kick(self, ctx, member: discord.Member, *, reason: str = "No reason provided"):
        try:
            await member.kick(reason=reason)
            await ctx.send(f"👢 Kicked {member}.")
            await log_action(ctx.guild, mod_embed("👢 Kick", discord.Color.orange(),
                user=f"{member} (`{member.id}`)", moderator=str(ctx.author), reason=reason))
        except discord.Forbidden:
            await ctx.send("❌ Missing permissions.")

    @commands.command()
    @mod_only()
    async def softban(self, ctx, member: discord.Member, *, reason: str = "No reason provided"):
        """Ban + unban to wipe 24hr message history."""
        try:
            await ctx.guild.ban(member, reason=f"Softban: {reason}", delete_message_seconds=86400)
            await ctx.guild.unban(member, reason="Softban complete")
            await ctx.send(f"🧹 Softbanned {member} (messages wiped).")
            await log_action(ctx.guild, mod_embed("🧹 Softban", discord.Color.orange(),
                user=f"{member} (`{member.id}`)", moderator=str(ctx.author), reason=reason))
        except discord.Forbidden:
            await ctx.send("❌ Missing permissions.")

    @commands.command()
    @mod_only()
    async def mute(self, ctx, member: discord.Member, minutes: int = 60, *, reason: str = "No reason provided"):
        try:
            await member.timeout(datetime.timedelta(minutes=minutes), reason=reason)
            await ctx.send(f"🔇 Muted {member} for {minutes} minutes.")
            await log_action(ctx.guild, mod_embed("🔇 Mute", discord.Color.blue(),
                user=f"{member} (`{member.id}`)", moderator=str(ctx.author),
                duration=f"{minutes} minutes", reason=reason))
        except discord.Forbidden:
            await ctx.send("❌ Missing permissions.")

    @commands.command()
    @mod_only()
    async def unmute(self, ctx, member: discord.Member):
        try:
            await member.timeout(None)
            await ctx.send(f"🔊 Unmuted {member}.")
        except discord.Forbidden:
            await ctx.send("❌ Missing permissions.")

    @commands.command()
    @mod_only()
    async def purge(self, ctx, amount: int, member: discord.Member = None):
        """!purge 20 OR !purge 20 @user"""
        if amount < 1 or amount > 500:
            return await ctx.send("❌ Amount must be 1-500.", delete_after=5)
        if member:
            check = lambda m: m.author.id == member.id
        else:
            check = lambda m: True
        deleted = await ctx.channel.purge(limit=amount, check=check)
        await ctx.send(f"🗑️ Deleted {len(deleted)} messages.", delete_after=4)

    @commands.command()
    @mod_only()
    async def slowmode(self, ctx, seconds: int = 0):
        """Set slowmode. 0 to disable."""
        await ctx.channel.edit(slowmode_delay=seconds)
        msg = f"✅ Slowmode set to {seconds}s." if seconds else "✅ Slowmode disabled."
        await ctx.send(msg, delete_after=5)

    @commands.command()
    @mod_only()
    async def unban(self, ctx, user_id: int, *, reason: str = "No reason provided"):
        try:
            user = await self.bot.fetch_user(user_id)
            await ctx.guild.unban(user, reason=reason)
            await ctx.send(f"✅ Unbanned {user}.")
        except discord.NotFound:
            await ctx.send("❌ User not found or not banned.")
        except discord.Forbidden:
            await ctx.send("❌ Missing permissions.")

async def setup(bot):
    await bot.add_cog(ModCmds(bot))