# Carl-Bot Suite Design Specification for ThemisBot

## Overview

Upgrade ThemisBot to feature a complete suite of Carl-bot level moderation, automation, utility, and welcome/goodbye capabilities.

Key Features:
1. **Command Prefix**: Global switch from `!` to `$` for all commands (`$help`, `$ban`, `$kick`, `$mute`, `$purge`, `$warn`, `$welcome`, `$goodbye`, `$rr`, `$embed`, `$userinfo`, `$serverinfo`, `$lock`, `$unlock`).
2. **Reaction Roles (`$rr`, `/rr`)**: Persistent reaction-to-role assignment using Discord emoji reactions and SQLite DB.
3. **Custom Embed Builder (`$embed`, `/embed`)**: Command and modal interface to create and post rich custom embeds to any channel.
4. **Utility & Info Commands (`$userinfo`, `$serverinfo`, `$avatar`, `$roleinfo`)**: Detailed server and member analytics embeds.
5. **Channel Management (`$lock`, `$unlock`)**: Quick channel lockdown toggling `@everyone` `SendMessages` permissions.
6. **Goodbye System (`$goodbye`, `/goodbye`)**: Member leave event listener (`GUILD_MEMBER_REMOVAL`) with customizable embeds.

---

## Data Model & Persistence (`src/db.rs`)

### New SQLite Tables

```sql
-- Reaction Roles Table
CREATE TABLE IF NOT EXISTS reaction_roles (
    guild_id   INTEGER NOT NULL,
    channel_id INTEGER NOT NULL,
    message_id INTEGER NOT NULL,
    emoji      TEXT    NOT NULL,
    role_id    INTEGER NOT NULL,
    PRIMARY KEY (guild_id, message_id, emoji)
);

-- Goodbye Config Table
CREATE TABLE IF NOT EXISTS goodbye_config (
    guild_id   INTEGER PRIMARY KEY,
    channel_id INTEGER NOT NULL,
    title      TEXT,
    message    TEXT NOT NULL,
    image_url  TEXT,
    enabled    INTEGER NOT NULL DEFAULT 1
);
```

### New Database Helpers (`src/db.rs`)
- `add_reaction_role(pool: &SqlitePool, guild_id: i64, channel_id: i64, message_id: i64, emoji: &str, role_id: i64)`
- `remove_reaction_role(pool: &SqlitePool, guild_id: i64, message_id: i64, emoji: &str)`
- `get_reaction_role(pool: &SqlitePool, guild_id: i64, message_id: i64, emoji: &str) -> Result<Option<i64>>`
- `list_reaction_roles(pool: &SqlitePool, guild_id: i64) -> Result<Vec<ReactionRoleRow>>`
- `get_goodbye_config(pool: &SqlitePool, guild_id: i64) -> Result<Option<GoodbyeConfig>>`
- `set_goodbye_channel(pool: &SqlitePool, guild_id: i64, channel_id: i64)`
- `set_goodbye_text(pool: &SqlitePool, guild_id: i64, text: &str)`
- `toggle_goodbye(pool: &SqlitePool, guild_id: i64) -> Result<bool>`

---

## Handlers & Modules

### 1. Reaction Roles Handler (`src/handlers/reaction_roles.rs`)
- Listens to `reaction_add` and `reaction_remove` gateway events.
- Queries `reaction_roles` table in SQLite for matching `(guild_id, message_id, emoji)`.
- If match found, calls `ctx.http.add_member_role` / `ctx.http.remove_member_role`.

### 2. Utility & Info Module (`src/handlers/commands/utility.rs`)
- **`$userinfo [@user]`**: Embed with joined date, created date, roles list, avatar, and top permissions.
- **`$serverinfo`**: Embed with server owner, creation date, member count (humans/bots), boost level, verification level, role count, channel count.
- **`$avatar [@user]`**: Displays avatar embed with direct image links.
- **`$roleinfo @role`**: Displays role ID, color, member count, position, and permission flags.
- **`$lock [#channel] [reason]`**: Edits channel `@everyone` permission overwrite to deny `SendMessages`.
- **`$unlock [#channel]`**: Restores `@everyone` `SendMessages` permission overwrite.

### 3. Embed Builder (`src/handlers/commands/embed.rs`)
- **`$embed #channel title="..." desc="..." color="#FFB6C1" image="..."`**: Parses arguments or json string and posts formatted embed.

### 4. Goodbye Handler (`src/handlers/goodbye.rs`)
- Listens to `guild_member_removal` event in `main.rs`.
- Posts customized goodbye embed replacing `{user}`, `{server}`, `{member_count}`.

---

## Commands Summary (`$` Prefix & Slash Commands)

All moderation and configuration commands require `check_user_mod` permission verification.

| Command | Usage Example | Description |
|---|---|---|
| Prefix Change | `$help`, `$ban`, `$kick`, `$warn` | Default prefix changed from `!` to `$`. |
| `$rr add` | `$rr add #roles 12345678 🎮 @Gamer` | Adds reaction role to message. |
| `$rr remove` | `$rr remove #roles 12345678 🎮` | Removes reaction role. |
| `$rr list` | `$rr list` | Displays active reaction roles. |
| `$embed` | `$embed #announcements title="Rules" desc="..."` | Constructs and sends custom embed. |
| `$userinfo` | `$userinfo @SamBlaze` | Displays user details and account info. |
| `$serverinfo` | `$serverinfo` | Displays server analytics and info. |
| `$avatar` | `$avatar @SamBlaze` | Displays high-res user avatar. |
| `$lock` | `$lock #general Spam attack lockdown` | Locks text channel for `@everyone`. |
| `$unlock` | `$unlock #general` | Unlocks text channel. |
| `$goodbye` | `$goodbye channel #leave` | Sets goodbye channel and template. |

---

## Testing & Verification Plan

1. **Build Check**: `cargo check` and `cargo test` clean build verification.
2. **Prefix Verification**: Verify commands respond to `$` prefix (`$help`, `$userinfo`, `$serverinfo`).
3. **Database & Event Verification**: Test reaction roles adding/removing roles on emoji click, `$lock` updating permissions, and `$embed` rendering.
