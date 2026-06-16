"""
ModBot - Multipurpose Security-First Discord Moderation Bot
============================================================
Cogs:
  - honeypot   : softban + server-wide message wipe on honeypot trigger
  - antispam   : rate-limit spam, anti-invite, anti-raid
  - modcmds    : ban, kick, mute, warn, purge, unwarn
  - warns      : persistent warn system with escalation
  - logging    : join/leave/edit/delete/ban logging
  - autorole   : assign role on member join

Requirements:
    pip install discord.py python-dotenv aiohttp aiosqlite

Privileged Intents required (Discord Developer Portal → Bot):
    - MESSAGE CONTENT INTENT
    - SERVER MEMBERS INTENT

Bot Permissions:
    Administrator (easiest) or manually:
    Ban Members, Kick Members, Manage Roles, Manage Messages,
    Read Message History, View Audit Log
"""

import discord
from discord.ext import commands
import os
import logging
import re
import asyncio
from dotenv import load_dotenv
from aiohttp import web

# ── Logging ───────────────────────────────────────────────────────────────────
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[
        logging.FileHandler("modbot.log", encoding="utf-8"),
        logging.StreamHandler()
    ]
)
log = logging.getLogger("modbot")

# ── Env ───────────────────────────────────────────────────────────────────────
load_dotenv()

def _require(key: str) -> str:
    val = os.getenv(key, "").strip()
    if not val:
        raise EnvironmentError(f"Missing required env var: {key}")
    return val

TOKEN   = _require("DISCORD_TOKEN")
GUILD_ID = int(_require("GUILD_ID"))

if not re.match(r'^[A-Za-z0-9_\-\.]{50,100}$', TOKEN):
    raise ValueError("DISCORD_TOKEN looks malformed.")

# ── Health server ─────────────────────────────────────────────────────────────
async def health(_req):
    return web.Response(text="OK")

async def start_health():
    app = web.Application()
    app.router.add_get("/health", health)
    runner = web.AppRunner(app)
    await runner.setup()
    port = int(os.getenv("PORT", "8085"))
    await web.TCPSite(runner, "0.0.0.0", port).start()
    log.info(f"Health server on :{port}")

# ── Bot ───────────────────────────────────────────────────────────────────────
intents = discord.Intents.default()
intents.message_content = True
intents.members = True

bot = commands.Bot(command_prefix="!", intents=intents)

COGS = ["cogs.honeypot", "cogs.antispam", "cogs.cmds", "cogs.warns", "cogs.logging", "cogs.role"]

@bot.event
async def on_ready():
    log.info(f"Logged in as {bot.user} (ID: {bot.user.id})")
    await start_health()

async def main():
    async with bot:
        for cog in COGS:
            try:
                await bot.load_extension(cog)
                log.info(f"Loaded {cog}")
            except Exception as e:
                log.error(f"Failed to load {cog}: {e}")
        await bot.start(TOKEN, reconnect=True)

if __name__ == "__main__":
    asyncio.run(main())