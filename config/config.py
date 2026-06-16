"""
Shared config loaded from .env
All cogs import from here — single source of truth.
"""
import os
from dotenv import load_dotenv

load_dotenv()

def _int(key, default=0):
    try:
        return int(os.getenv(key, default))
    except ValueError:
        return default

def _set(key):
    raw = os.getenv(key, "")
    return set(int(x.strip()) for x in raw.split(",") if x.strip().isdigit())

def _bool(key, default=False):
    return os.getenv(key, str(default)).lower() in ("1", "true", "yes")

# ── Core ──────────────────────────────────────────────────────────────────────
GUILD_ID          = _int("GUILD_ID")
MOD_LOG_CHANNEL   = _int("MOD_LOG_CHANNEL_ID")
IMMUNE_ROLE_IDS   = _set("IMMUNE_ROLE_IDS")   # roles never punished
MOD_ROLE_IDS      = _set("MOD_ROLE_IDS")       # roles that can use mod commands

# ── Honeypot ──────────────────────────────────────────────────────────────────
HONEYPOT_CHANNEL  = _int("HONEYPOT_CHANNEL_ID")

# ── Antispam ──────────────────────────────────────────────────────────────────
SPAM_MSG_LIMIT    = _int("SPAM_MSG_LIMIT", 5)       # max messages per window
SPAM_WINDOW_SECS  = _int("SPAM_WINDOW_SECS", 5)     # window in seconds
ANTI_INVITE       = _bool("ANTI_INVITE", True)       # block Discord invites
RAID_JOIN_LIMIT   = _int("RAID_JOIN_LIMIT", 10)      # joins per window = raid
RAID_WINDOW_SECS  = _int("RAID_WINDOW_SECS", 10)    # raid detection window

# ── Warns ─────────────────────────────────────────────────────────────────────
WARN_MUTE_AT      = _int("WARN_MUTE_AT", 3)         # mute after N warns
WARN_KICK_AT      = _int("WARN_KICK_AT", 5)         # kick after N warns
WARN_BAN_AT       = _int("WARN_BAN_AT", 7)          # ban after N warns
MUTE_ROLE_ID      = _int("MUTE_ROLE_ID")            # muted role ID

# ── Autorole ──────────────────────────────────────────────────────────────────
AUTOROLE_ID       = _int("AUTOROLE_ID")             # role to assign on join

# ── DB ────────────────────────────────────────────────────────────────────────
DB_PATH           = os.getenv("DB_PATH", "modbot.db")