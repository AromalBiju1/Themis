# Welcome System Design Specification

## Overview

ThemisBot will feature a dynamic, per-guild Welcome Message system built on Serenity 0.12 and SQLite (`modbot.db`). When a new member joins a Discord server (`GUILD_MEMBER_ADDITION`), ThemisBot automatically constructs and posts a rich welcome embed to a designated channel.

The welcome embed matches the Mimu aesthetic:
- **Outside Content**: Ping mention (`<@UserId>`) to notify the joining user.
- **Thumbnail**: Joining user's avatar (`member.user.face()`).
- **Description**: Custom welcome text with bullet points, emojis, and placeholders (`{user}`, `{server}`, `{member_count}`).
- **Banner Image**: Bottom image attachment / URL.
- **Footer**: `Now we have {member_count} members !! | <timestamp>`.
- **Color**: `#FFB6C1` (Soft aesthetic pink).

Server administrators and moderators can configure and test welcome messages using either **Discord Slash Commands** (`/welcome`, `/welcometest`) or **Prefix Commands** (`!welcome`, `!welcometest`).

---

## Data Model & Persistence (`src/db.rs`)

Welcome configurations are stored per-guild in SQLite:

```sql
CREATE TABLE IF NOT EXISTS welcome_config (
    guild_id   INTEGER PRIMARY KEY,
    channel_id INTEGER NOT NULL,
    title      TEXT,
    message    TEXT NOT NULL,
    image_url  TEXT,
    enabled    INTEGER NOT NULL DEFAULT 1
);
```

### Database Helpers (`src/db.rs`)
- `init_welcome_db(pool: &SqlitePool)`: Creates `welcome_config` table on database startup.
- `get_welcome_config(pool: &SqlitePool, guild_id: i64) -> anyhow::Result<Option<WelcomeConfig>>`
- `set_welcome_channel(pool: &SqlitePool, guild_id: i64, channel_id: i64) -> anyhow::Result<()>`
- `set_welcome_text(pool: &SqlitePool, guild_id: i64, message: &str) -> anyhow::Result<()>`
- `set_welcome_image(pool: &SqlitePool, guild_id: i64, image_url: &str) -> anyhow::Result<()>`
- `toggle_welcome(pool: &SqlitePool, guild_id: i64) -> anyhow::Result<bool>`

---

## Welcome Handler (`src/handlers/welcome.rs`)

### `handle_member_join(ctx: &Context, member: &Member)`
1. Retrieves `SqlitePool` from `ctx.data` TypeMap (`BotData`).
2. Fetches `WelcomeConfig` for `member.guild_id`.
3. If `enabled == 1` and `channel_id > 0`:
   - Calculates current guild member count from `ctx.cache` or HTTP.
   - Formats description text by replacing:
     - `{user}` → `<@{user_id}>`
     - `{server}` → Guild name
     - `{member_count}` → Total member count
   - Constructs `CreateEmbed`:
     - Description: Formatted text
     - Thumbnail: `member.user.face()`
     - Image: `image_url` (if set and non-empty)
     - Footer: `Now we have {member_count} members !!`
     - Timestamp: Current UTC timestamp
     - Color: `Color::from_rgb(255, 182, 193)` (`#FFB6C1`)
   - Sends `CreateMessage::new().content(format!("<@{}>", member.user.id)).embed(embed)` to `ChannelId::new(channel_id as u64)`.

---

## Commands Interface (`src/handlers/commands/welcome.rs`)

Both Slash Commands and Prefix Commands require Moderator / Admin permissions (`check_mod`).

### Slash Commands (`/welcome`, `/welcometest`)
Registered during bot `ready` event:
- `/welcome set_channel [channel]` - Select destination channel via Discord channel picker.
- `/welcome set_text [message]` - Update welcome description text.
- `/welcome set_image [url]` - Update banner image URL.
- `/welcome toggle` - Enable / Disable welcome messages.
- `/welcome status` - Display current welcome configuration embed.
- `/welcometest` - Post a live preview of the welcome embed in the designated channel.

### Prefix Commands (`!welcome`, `!welcometest`)
- `!welcome channel #channel`
- `!welcome text <message>`
- `!welcome image <url>`
- `!welcome toggle`
- `!welcome status`
- `!welcometest`

---

## Testing & Verification Plan

1. **Database Schema Verification**: Run `cargo check` and verify `welcome_config` table creation during `init_db`.
2. **Command Verification**: Execute `!welcome channel`, `!welcome text`, `!welcome image`, and `/welcometest` to verify database persistence and embed rendering.
3. **Event Verification**: Simulate member join or call `!welcometest` to ensure the embed renders correctly with thumbnail, placeholders, banner image, and footer.
