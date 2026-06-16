"""
Shared utilities used across cogs.
"""
import discord
from datetime import datetime, timezone
from config.config import IMMUNE_ROLE_IDS, MOD_ROLE_IDS, MOD_LOG_CHANNEL

def utcnow() -> datetime:
    return datetime.now(timezone.utc)

def is_immune(member: discord.Member) -> bool:
    """True if member should never be auto-punished."""
    if member.bot:
        return True
    if member.guild_permissions.administrator:
        return True
    return bool(IMMUNE_ROLE_IDS & {r.id for r in member.roles})

def is_mod(member: discord.Member) -> bool:
    """True if member can use mod commands."""
    if member.guild_permissions.administrator:
        return True
    return bool(MOD_ROLE_IDS & {r.id for r in member.roles})

def mod_embed(title: str, color: discord.Color, **fields) -> discord.Embed:
    embed = discord.Embed(title=title, color=color, timestamp=utcnow())
    for name, value in fields.items():
        embed.add_field(name=name.replace("_", " ").title(), value=str(value), inline=False)
    embed.set_footer(text="Themis")
    return embed

async def log_action(guild: discord.Guild, embed: discord.Embed):
    ch = guild.get_channel(MOD_LOG_CHANNEL)
    if ch:
        try:
            await ch.send(embed=embed)
        except discord.Forbidden:
            pass