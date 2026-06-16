# Themis — Multipurpose Discord Moderation Bot

## Features
| Cog | What it does |
|-----|-------------|
| 🍯 Honeypot | Softban anyone who posts in the trap channel, wipes 24hr messages server-wide |
| 🚫 Antispam | Rate-limit spam → auto timeout, block invite links, raid detection → lockdown |
| ⚠️ Warns | Persistent warn system with auto-escalation (mute → kick → ban) |
| 🔨 Mod Commands | !ban !kick !softban !mute !unmute !purge !slowmode !unban |
| 📋 Logging | Join/leave/edit/delete/ban events logged to mod channel |
| 👋 Autorole | Auto-assign a role to new members |

## Setup

### 1. Install dependencies
```bash
pip install -r requirements.txt
```

### 2. Configure
```bash
cp .env.example .env
# Fill in your values
```

### 3. Discord Developer Portal
- Bot → Privileged Gateway Intents:
  - ✅ MESSAGE CONTENT INTENT
  - ✅ SERVER MEMBERS INTENT
- Bot permissions: Administrator (or Ban, Kick, Manage Roles, Manage Messages, Read History)

### 4. Run
```bash
python main.py
```

## Commands
| Command | Description |
|---------|-------------|
| `!ban @user [reason]` | Ban a member |
| `!kick @user [reason]` | Kick a member |
| `!softban @user [reason]` | Softban (ban+unban, wipes 24hr messages) |
| `!mute @user [minutes] [reason]` | Timeout a member (default 60min) |
| `!unmute @user` | Remove timeout |
| `!warn @user [reason]` | Warn a member (escalates at thresholds) |
| `!warns @user` | List warns for a member |
| `!unwarn [id]` | Remove a specific warn by ID |
| `!clearwarns @user` | Clear all warns for a member |
| `!purge [amount] [@user]` | Purge messages (optional: from specific user) |
| `!slowmode [seconds]` | Set slowmode (0 to disable) |
| `!unban [user_id]` | Unban a user |
| `!raidoff` | Manually disable raid mode |

## Hosting (Render + UptimeRobot)
1. Push to GitHub
2. Create a Web Service on Render, set env vars
3. Add UptimeRobot monitor → `https://your-render-url/health` every 5 mins
