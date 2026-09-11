# Carl-Bot Suite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform ThemisBot into a full Carl-bot level Discord bot suite with `$` prefix, Reaction Roles, Custom Embed Builder, Utility & Server/User Analytics, Channel Lockdown, and Goodbye event handling.

**Architecture:** Switch default prefix to `$`, add SQLite persistence for reaction roles and goodbye settings, create dedicated handlers for reaction events (`reaction_add`/`reaction_remove`), and implement commands for `$rr`, `$embed`, `$userinfo`, `$serverinfo`, `$avatar`, `$roleinfo`, `$lock`, `$unlock`, and `$goodbye`.

**Tech Stack:** Rust, Serenity 0.12, SQLx (SQLite), Tokio, Tracing.

## Global Constraints

- Change default command prefix from `!` to `$` in `src/main.rs`.
- Use `crate::utils::check_user_mod` for all moderator/admin command permission checks.
- Compile cleanly with `cargo check` and `cargo test`.

---

### Task 1: Command Prefix Update (`src/main.rs`)

**Files:**
- Modify: `src/main.rs:198-208`

**Interfaces:**
- Consumes: `serenity::framework::standard::Configuration`
- Produces: `$` prefix for all standard framework commands

- [ ] **Step 1: Update prefix in `src/main.rs`**

Change `prefix("!")` to `prefix("$")` in `src/main.rs`:

```rust
framework.configure(Configuration::new().prefix("$"));
```

- [ ] **Step 2: Run `cargo check` to verify**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat(config): change default command prefix from ! to $"
```

---

### Task 2: Database Schema & Query Helpers for Reaction Roles & Goodbye (`src/db.rs`)

**Files:**
- Modify: `src/db.rs`

**Interfaces:**
- Consumes: `sqlx::SqlitePool`
- Produces: `reaction_roles` and `goodbye_config` tables and helper functions.

- [ ] **Step 1: Add table schemas to `init_db` in `src/db.rs`**

```rust
sqlx::query(
    "CREATE TABLE IF NOT EXISTS reaction_roles (
        guild_id   INTEGER NOT NULL,
        channel_id INTEGER NOT NULL,
        message_id INTEGER NOT NULL,
        emoji      TEXT    NOT NULL,
        role_id    INTEGER NOT NULL,
        PRIMARY KEY (guild_id, message_id, emoji)
    )"
)
.execute(pool)
.await?;

sqlx::query(
    "CREATE TABLE IF NOT EXISTS goodbye_config (
        guild_id   INTEGER PRIMARY KEY,
        channel_id INTEGER NOT NULL,
        title      TEXT,
        message    TEXT NOT NULL,
        image_url  TEXT,
        enabled    INTEGER NOT NULL DEFAULT 1
    )"
)
.execute(pool)
.await?;
```

- [ ] **Step 2: Add database helper functions for reaction roles and goodbye in `src/db.rs`**

```rust
pub async fn add_reaction_role(
    pool: &SqlitePool,
    guild_id: i64,
    channel_id: i64,
    message_id: i64,
    emoji: &str,
    role_id: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO reaction_roles (guild_id, channel_id, message_id, emoji, role_id) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(guild_id, message_id, emoji) DO UPDATE SET role_id=EXCLUDED.role_id"
    )
    .bind(guild_id)
    .bind(channel_id)
    .bind(message_id)
    .bind(emoji)
    .bind(role_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_reaction_role(
    pool: &SqlitePool,
    guild_id: i64,
    message_id: i64,
    emoji: &str,
) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM reaction_roles WHERE guild_id=? AND message_id=? AND emoji=?")
        .bind(guild_id)
        .bind(message_id)
        .bind(emoji)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_reaction_role(
    pool: &SqlitePool,
    guild_id: i64,
    message_id: i64,
    emoji: &str,
) -> anyhow::Result<Option<i64>> {
    let row = sqlx::query(
        "SELECT role_id FROM reaction_roles WHERE guild_id=? AND message_id=? AND emoji=?"
    )
    .bind(guild_id)
    .bind(message_id)
    .bind(emoji)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.get::<i64, _>("role_id")))
}

#[derive(Debug, Clone)]
pub struct GoodbyeConfig {
    pub guild_id: i64,
    pub channel_id: i64,
    pub title: Option<String>,
    pub message: String,
    pub image_url: Option<String>,
    pub enabled: bool,
}

pub async fn get_goodbye_config(pool: &SqlitePool, guild_id: i64) -> anyhow::Result<Option<GoodbyeConfig>> {
    let row = sqlx::query(
        "SELECT guild_id, channel_id, title, message, image_url, enabled FROM goodbye_config WHERE guild_id=?"
    )
    .bind(guild_id)
    .fetch_optional(pool)
    .await?;

    if let Some(r) = row {
        Ok(Some(GoodbyeConfig {
            guild_id: r.get("guild_id"),
            channel_id: r.get("channel_id"),
            title: r.get("title"),
            message: r.get("message"),
            image_url: r.get("image_url"),
            enabled: r.get::<i64, _>("enabled") != 0,
        }))
    } else {
        Ok(None)
    }
}

pub async fn set_goodbye_channel(pool: &SqlitePool, guild_id: i64, channel_id: i64) -> anyhow::Result<()> {
    let default_msg = "👋 Goodbye {user}! We hope to see you again soon in {server}.";
    sqlx::query(
        "INSERT INTO goodbye_config (guild_id, channel_id, message, enabled) VALUES (?, ?, ?, 1)
         ON CONFLICT(guild_id) DO UPDATE SET channel_id=EXCLUDED.channel_id"
    )
    .bind(guild_id)
    .bind(channel_id)
    .bind(default_msg)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_goodbye_text(pool: &SqlitePool, guild_id: i64, text: &str) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO goodbye_config (guild_id, channel_id, message, enabled) VALUES (?, 0, ?, 1)
         ON CONFLICT(guild_id) DO UPDATE SET message=EXCLUDED.message"
    )
    .bind(guild_id)
    .bind(text)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn toggle_goodbye(pool: &SqlitePool, guild_id: i64) -> anyhow::Result<bool> {
    let current = get_goodbye_config(pool, guild_id).await?.map(|c| c.enabled).unwrap_or(true);
    let new_val = if current { 0 } else { 1 };
    sqlx::query("UPDATE goodbye_config SET enabled=? WHERE guild_id=?")
        .bind(new_val)
        .bind(guild_id)
        .execute(pool)
        .await?;
    Ok(new_val == 1)
}
```

- [ ] **Step 3: Verify build with `cargo check`**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/db.rs
git commit -m "feat(db): add reaction_roles and goodbye_config tables"
```

---

### Task 3: Reaction Roles Handler & Command (`src/handlers/reaction_roles.rs` & `src/handlers/commands/rr.rs`)

**Files:**
- Create: `src/handlers/reaction_roles.rs`
- Create: `src/handlers/commands/rr.rs`
- Modify: `src/handlers/mod.rs`
- Modify: `src/handlers/commands/mod.rs`

**Interfaces:**
- Consumes: `serenity::all::Reaction`, `serenity::all::Context`
- Produces: `handle_reaction_add`, `handle_reaction_remove`, `RR_GROUP` (`$rr`)

- [ ] **Step 1: Create `src/handlers/reaction_roles.rs`**

```rust
use serenity::all::*;
use crate::{BotData, db};

pub async fn handle_reaction_add(ctx: &Context, reaction: &Reaction) {
    let guild_id = match reaction.guild_id {
        Some(g) => g,
        None => return,
    };
    let user_id = match reaction.user_id {
        Some(u) => u,
        None => return,
    };
    if let Some(current_user) = ctx.cache.current_user_id() {
        if user_id == current_user {
            return;
        }
    }

    let emoji_str = reaction.emoji.to_string();
    let pool = {
        let data = ctx.data.read().await;
        data.get::<BotData>().unwrap().db.clone()
    };

    if let Ok(Some(role_id)) = db::get_reaction_role(&pool, guild_id.get() as i64, reaction.message_id.get() as i64, &emoji_str).await {
        let role = RoleId::new(role_id as u64);
        if let Err(e) = ctx.http.add_member_role(guild_id, user_id, role, Some("Reaction role")).await {
            tracing::error!("reaction_roles: failed to add role: {e}");
        }
    }
}

pub async fn handle_reaction_remove(ctx: &Context, reaction: &Reaction) {
    let guild_id = match reaction.guild_id {
        Some(g) => g,
        None => return,
    };
    let user_id = match reaction.user_id {
        Some(u) => u,
        None => return,
    };

    let emoji_str = reaction.emoji.to_string();
    let pool = {
        let data = ctx.data.read().await;
        data.get::<BotData>().unwrap().db.clone()
    };

    if let Ok(Some(role_id)) = db::get_reaction_role(&pool, guild_id.get() as i64, reaction.message_id.get() as i64, &emoji_str).await {
        let role = RoleId::new(role_id as u64);
        if let Err(e) = ctx.http.remove_member_role(guild_id, user_id, role, Some("Reaction role removed")).await {
            tracing::error!("reaction_roles: failed to remove role: {e}");
        }
    }
}
```

- [ ] **Step 2: Create `src/handlers/commands/rr.rs`**

```rust
use serenity::{
    all::*,
    framework::standard::{macros::{command, group}, Args, CommandResult},
};
use crate::{BotData, db, utils::check_user_mod};

#[group]
#[commands(rr)]
pub struct RRCmds;

#[command]
#[description = "Reaction Roles: $rr add #ch <msg_id> <emoji> @role | $rr remove #ch <msg_id> <emoji>"]
pub async fn rr(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let bot_data = data.get::<BotData>().unwrap();
    let cfg = &bot_data.config;

    if !check_user_mod(ctx, msg, cfg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }

    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return Ok(()),
    };

    let sub = args.single::<String>().unwrap_or_default().to_lowercase();
    let pool = &bot_data.db;

    match sub.as_str() {
        "add" => {
            let channel_id = match args.single::<ChannelId>() {
                Ok(ch) => ch,
                Err(_) => { msg.reply(&ctx.http, "❌ Provide a channel mention (e.g. `#roles`).").await?; return Ok(()); }
            };
            let msg_id = match args.single::<u64>() {
                Ok(id) => MessageId::new(id),
                Err(_) => { msg.reply(&ctx.http, "❌ Provide a valid message ID.").await?; return Ok(()); }
            };
            let emoji_str = match args.single::<String>() {
                Ok(e) => e,
                Err(_) => { msg.reply(&ctx.http, "❌ Provide an emoji.").await?; return Ok(()); }
            };
            let role_id = match args.single::<RoleId>() {
                Ok(r) => r,
                Err(_) => { msg.reply(&ctx.http, "❌ Provide a role mention or ID.").await?; return Ok(()); }
            };

            db::add_reaction_role(pool, guild_id.get() as i64, channel_id.get() as i64, msg_id.get() as i64, &emoji_str, role_id.get() as i64).await?;
            
            // Add initial reaction from bot
            if let Ok(reaction_emoji) = ReactionType::try_from(emoji_str.as_str()) {
                let _ = ctx.http.create_reaction(channel_id, msg_id, &reaction_emoji).await;
            }

            msg.reply(&ctx.http, format!("✅ Added reaction role: {} -> <@&{}> on message `{}`.", emoji_str, role_id, msg_id)).await?;
        }
        "remove" | "delete" => {
            let _channel_id = match args.single::<ChannelId>() {
                Ok(ch) => ch,
                Err(_) => { msg.reply(&ctx.http, "❌ Provide a channel mention.").await?; return Ok(()); }
            };
            let msg_id = match args.single::<u64>() {
                Ok(id) => id,
                Err(_) => { msg.reply(&ctx.http, "❌ Provide a valid message ID.").await?; return Ok(()); }
            };
            let emoji_str = match args.single::<String>() {
                Ok(e) => e,
                Err(_) => { msg.reply(&ctx.http, "❌ Provide an emoji.").await?; return Ok(()); }
            };

            db::remove_reaction_role(pool, guild_id.get() as i64, msg_id as i64, &emoji_str).await?;
            msg.reply(&ctx.http, format!("✅ Removed reaction role for {} on message `{}`.", emoji_str, msg_id)).await?;
        }
        _ => {
            msg.reply(
                &ctx.http,
                "ℹ️ Usage:\n• `$rr add #channel <message_id> <emoji> @role` - Link emoji reaction to role\n• `$rr remove #channel <message_id> <emoji>` - Remove reaction role",
            ).await?;
        }
    }

    Ok(())
}
```

- [ ] **Step 3: Export in `src/handlers/mod.rs` and `src/handlers/commands/mod.rs`**

- [ ] **Step 4: Verify build with `cargo check`**

Run: `cargo check`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/handlers/reaction_roles.rs src/handlers/commands/rr.rs src/handlers/mod.rs src/handlers/commands/mod.rs
git commit -m "feat(rr): add reaction roles handler and $rr command"
```

---

### Task 4: Utility & Info Commands (`src/handlers/commands/utility.rs`)

**Files:**
- Create: `src/handlers/commands/utility.rs`
- Modify: `src/handlers/commands/mod.rs`

**Interfaces:**
- Consumes: `serenity::all::*`
- Produces: `UTILITY_GROUP` (`$userinfo`, `$serverinfo`, `$avatar`, `$roleinfo`, `$lock`, `$unlock`)

- [ ] **Step 1: Create `src/handlers/commands/utility.rs`**

```rust
use serenity::{
    all::*,
    framework::standard::{macros::{command, group}, Args, CommandResult},
};
use chrono::Utc;
use crate::utils::check_user_mod;

#[group]
#[commands(userinfo, serverinfo, avatar, roleinfo, lock, unlock)]
pub struct UtilityCmds;

#[command]
#[description = "Display user details, join date, account creation date, and roles."]
pub async fn userinfo(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let target_user = if let Ok(parsed_id) = crate::utils::parse_target_from_args(&mut args).map(UserId::new) {
        ctx.http.get_user(parsed_id).await.unwrap_or_else(|_| msg.author.clone())
    } else {
        msg.author.clone()
    };

    let guild_id = msg.guild_id.unwrap_or_default();
    let member = ctx.http.get_member(guild_id, target_user.id).await.ok();

    let created_at = format!("<t:{}:F>", target_user.created_at().unix_timestamp());
    let joined_at = member.as_ref().and_then(|m| m.joined_at).map(|t| format!("<t:{}:F>", t.unix_timestamp())).unwrap_or_else(|| "Unknown".to_string());

    let roles_str = member.as_ref().map(|m| {
        let r: Vec<String> = m.roles.iter().map(|rid| format!("<@&{}>", rid)).collect();
        if r.is_empty() { "None".to_string() } else { r.join(", ") }
    }).unwrap_or_else(|| "None".to_string());

    let embed = CreateEmbed::new()
        .title(format!("👤 User Info: {}", target_user.tag()))
        .color(Color::BLURPLE)
        .thumbnail(target_user.face())
        .field("Mention", format!("<@{}>", target_user.id), true)
        .field("ID", target_user.id.to_string(), true)
        .field("Created At", created_at, false)
        .field("Joined Server", joined_at, false)
        .field("Roles", roles_str, false);

    msg.channel_id.send_message(&ctx.http, CreateMessage::new().embed(embed)).await?;
    Ok(())
}

#[command]
#[description = "Display server analytics, member count, creation date, and boost level."]
pub async fn serverinfo(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return Ok(()),
    };

    let guild = match ctx.http.get_guild(guild_id).await {
        Ok(g) => g,
        Err(_) => return Ok(()),
    };

    let created_at = format!("<t:{}:F>", guild.id.created_at().unix_timestamp());
    let owner_mention = format!("<@{}>", guild.owner_id);

    let embed = CreateEmbed::new()
        .title(format!("🏰 Server Info: {}", guild.name))
        .color(Color::GOLD)
        .thumbnail(guild.icon_url().unwrap_or_default())
        .field("Owner", owner_mention, true)
        .field("Members", guild.approximate_member_count.unwrap_or(0).to_string(), true)
        .field("Boost Tier", format!("{:?}", guild.premium_tier), true)
        .field("Created At", created_at, false);

    msg.channel_id.send_message(&ctx.http, CreateMessage::new().embed(embed)).await?;
    Ok(())
}

#[command]
#[description = "Display high-res user avatar."]
pub async fn avatar(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let target_user = if let Ok(parsed_id) = crate::utils::parse_target_from_args(&mut args).map(UserId::new) {
        ctx.http.get_user(parsed_id).await.unwrap_or_else(|_| msg.author.clone())
    } else {
        msg.author.clone()
    };

    let avatar_url = target_user.face();
    let embed = CreateEmbed::new()
        .title(format!("🖼️ Avatar: {}", target_user.tag()))
        .color(Color::LIGHT_GREY)
        .image(&avatar_url)
        .description(format!("[Direct Link]({})", avatar_url));

    msg.channel_id.send_message(&ctx.http, CreateMessage::new().embed(embed)).await?;
    Ok(())
}

#[command]
#[description = "Display role information and member count."]
pub async fn roleinfo(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let role_id = match args.single::<RoleId>() {
        Ok(r) => r,
        Err(_) => { msg.reply(&ctx.http, "❌ Provide a role mention or ID.").await?; return Ok(()); }
    };

    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return Ok(()),
    };

    if let Ok(guild) = ctx.http.get_guild(guild_id).await {
        if let Some(role) = guild.roles.get(&role_id) {
            let embed = CreateEmbed::new()
                .title(format!("🏷️ Role Info: {}", role.name))
                .color(role.color)
                .field("ID", role.id.to_string(), true)
                .field("Color", format!("#{:06X}", role.color.0), true)
                .field("Position", role.position.to_string(), true)
                .field("Mentionable", role.mentionable.to_string(), true)
                .field("Hoisted", role.hoist.to_string(), true);

            msg.channel_id.send_message(&ctx.http, CreateMessage::new().embed(embed)).await?;
            return Ok(());
        }
    }

    msg.reply(&ctx.http, "❌ Role not found.").await?;
    Ok(())
}

#[command]
#[description = "Lock down a text channel by revoking SendMessages for @everyone."]
pub async fn lock(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let cfg = &data.get::<crate::BotData>().unwrap().config;

    if !check_user_mod(ctx, msg, cfg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }

    let target_ch = args.single::<ChannelId>().unwrap_or(msg.channel_id);
    let reason = args.rest().trim();
    let reason_str = if reason.is_empty() { "Channel locked by moderator" } else { reason };

    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return Ok(()),
    };

    let everyone_role_id = RoleId::new(guild_id.get());
    let overwrite = PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::SEND_MESSAGES,
        kind: PermissionOverwriteType::Role(everyone_role_id),
    };

    if let Err(e) = target_ch.create_permission(&ctx.http, overwrite).await {
        msg.reply(&ctx.http, format!("❌ Lock failed: {e}")).await?;
    } else {
        msg.reply(&ctx.http, format!("🔒 Locked <#{}>. Reason: {}", target_ch, reason_str)).await?;
    }

    Ok(())
}

#[command]
#[description = "Unlock a text channel by restoring SendMessages for @everyone."]
pub async fn unlock(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let cfg = &data.get::<crate::BotData>().unwrap().config;

    if !check_user_mod(ctx, msg, cfg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }

    let target_ch = args.single::<ChannelId>().unwrap_or(msg.channel_id);
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return Ok(()),
    };

    let everyone_role_id = RoleId::new(guild_id.get());
    let overwrite = PermissionOverwrite {
        allow: Permissions::SEND_MESSAGES,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Role(everyone_role_id),
    };

    if let Err(e) = target_ch.create_permission(&ctx.http, overwrite).await {
        msg.reply(&ctx.http, format!("❌ Unlock failed: {e}")).await?;
    } else {
        msg.reply(&ctx.http, format!("🔓 Unlocked <#{}>.", target_ch)).await?;
    }

    Ok(())
}
```

- [ ] **Step 2: Export in `src/handlers/commands/mod.rs`**

- [ ] **Step 3: Verify build with `cargo check`**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/handlers/commands/utility.rs src/handlers/commands/mod.rs
git commit -m "feat(utility): add userinfo, serverinfo, avatar, roleinfo, lock, unlock commands"
```

---

### Task 5: Embed Builder Command (`src/handlers/commands/embed.rs`)

**Files:**
- Create: `src/handlers/commands/embed.rs`
- Modify: `src/handlers/commands/mod.rs`

**Interfaces:**
- Consumes: `serenity::all::*`
- Produces: `EMBED_GROUP` (`$embed`)

- [ ] **Step 1: Create `src/handlers/commands/embed.rs`**

```rust
use serenity::{
    all::*,
    framework::standard::{macros::{command, group}, Args, CommandResult},
};
use crate::utils::check_user_mod;

#[group]
#[commands(embed)]
pub struct EmbedCmds;

#[command]
#[description = "Send a custom embed: $embed #channel title=\"Title\" desc=\"Description\" color=\"#FFB6C1\" image=\"url\""]
pub async fn embed(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let cfg = &data.get::<crate::BotData>().unwrap().config;

    if !check_user_mod(ctx, msg, cfg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }

    let target_ch = match args.single::<ChannelId>() {
        Ok(ch) => ch,
        Err(_) => msg.channel_id,
    };

    let rest = args.rest().trim();
    if rest.is_empty() {
        msg.reply(&ctx.http, "ℹ️ Usage: `$embed #channel title=\"Rules\" desc=\"Welcome!\" color=\"#FFB6C1\"`").await?;
        return Ok(());
    }

    let mut title = String::new();
    let mut desc = String::new();
    let mut color_hex = String::new();
    let mut image_url = String::new();

    for part in rest.split_whitespace() {
        if let Some(val) = part.strip_prefix("title=\"").and_then(|s| s.strip_suffix('"')) {
            title = val.to_string();
        } else if let Some(val) = part.strip_prefix("desc=\"").and_then(|s| s.strip_suffix('"')) {
            desc = val.to_string();
        } else if let Some(val) = part.strip_prefix("color=\"").and_then(|s| s.strip_suffix('"')) {
            color_hex = val.to_string();
        } else if let Some(val) = part.strip_prefix("image=\"").and_then(|s| s.strip_suffix('"')) {
            image_url = val.to_string();
        }
    }

    if desc.is_empty() && title.is_empty() {
        desc = rest.to_string();
    }

    let color = if color_hex.starts_with('#') {
        u32::from_str_radix(&color_hex[1..], 16).map(Color::new).unwrap_or(Color::BLURPLE)
    } else {
        Color::BLURPLE
    };

    let mut embed = CreateEmbed::new().color(color);
    if !title.is_empty() { embed = embed.title(title); }
    if !desc.is_empty() { embed = embed.description(desc); }
    if !image_url.is_empty() { embed = embed.image(image_url); }

    target_ch.send_message(&ctx.http, CreateMessage::new().embed(embed)).await?;
    msg.reply(&ctx.http, format!("✅ Sent custom embed to <#{}>.", target_ch)).await?;
    Ok(())
}
```

- [ ] **Step 2: Export in `src/handlers/commands/mod.rs`**

- [ ] **Step 3: Verify build with `cargo check`**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/handlers/commands/embed.rs src/handlers/commands/mod.rs
git commit -m "feat(embed): add custom embed builder $embed command"
```

---

### Task 6: Goodbye Handler & Commands (`src/handlers/goodbye.rs` & `src/handlers/commands/goodbye.rs`)

**Files:**
- Create: `src/handlers/goodbye.rs`
- Create: `src/handlers/commands/goodbye.rs`
- Modify: `src/handlers/mod.rs`
- Modify: `src/handlers/commands/mod.rs`

**Interfaces:**
- Consumes: `serenity::all::*`
- Produces: `handle_member_leave`, `GOODBYE_GROUP` (`$goodbye`, `$goodbyetest`)

- [ ] **Step 1: Create `src/handlers/goodbye.rs`**

```rust
use serenity::all::*;
use crate::{BotData, db};

pub async fn handle_member_leave(ctx: &Context, guild_id: GuildId, user: &User, _member_data: &Option<Member>) {
    let pool = {
        let data = ctx.data.read().await;
        data.get::<BotData>().unwrap().db.clone()
    };

    let cfg = match db::get_goodbye_config(&pool, guild_id.get() as i64).await {
        Ok(Some(cfg)) => cfg,
        _ => return,
    };

    if !cfg.enabled || cfg.channel_id <= 0 {
        return;
    }

    let channel_id = ChannelId::new(cfg.channel_id as u64);
    send_goodbye_embed(ctx, guild_id, user, channel_id, &cfg).await;
}

pub async fn send_goodbye_embed(
    ctx: &Context,
    guild_id: GuildId,
    user: &User,
    channel_id: ChannelId,
    cfg: &db::GoodbyeConfig,
) {
    let server_name = ctx.cache.guild(guild_id).map(|g| g.name.clone()).unwrap_or_else(|| "Server".to_string());
    let formatted = cfg.message
        .replace("{user}", &user.tag())
        .replace("{server}", &server_name);

    let embed = CreateEmbed::new()
        .color(Color::DARK_RED)
        .description(formatted)
        .thumbnail(user.face());

    let _ = channel_id.send_message(&ctx.http, CreateMessage::new().embed(embed)).await;
}
```

- [ ] **Step 2: Create `src/handlers/commands/goodbye.rs`**

```rust
use serenity::{
    all::*,
    framework::standard::{macros::{command, group}, Args, CommandResult},
};
use crate::{BotData, db, utils::check_user_mod, handlers::goodbye::send_goodbye_embed};

#[group]
#[commands(goodbye, goodbyetest)]
pub struct GoodbyeCmds;

#[command]
#[description = "Configure goodbye settings: $goodbye channel #ch | text <msg> | toggle | status"]
pub async fn goodbye(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let bot_data = data.get::<BotData>().unwrap();
    let cfg = &bot_data.config;

    if !check_user_mod(ctx, msg, cfg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }

    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return Ok(()),
    };

    let sub = args.single::<String>().unwrap_or_default().to_lowercase();
    let pool = &bot_data.db;

    match sub.as_str() {
        "channel" | "ch" => {
            let target_ch = args.single::<ChannelId>().unwrap_or(msg.channel_id);
            db::set_goodbye_channel(pool, guild_id.get() as i64, target_ch.get() as i64).await?;
            msg.reply(&ctx.http, format!("✅ Goodbye channel set to <#{}>.", target_ch)).await?;
        }
        "text" | "msg" => {
            let text = args.rest().trim();
            if text.is_empty() {
                msg.reply(&ctx.http, "❌ Please provide goodbye text.").await?;
                return Ok(());
            }
            db::set_goodbye_text(pool, guild_id.get() as i64, text).await?;
            msg.reply(&ctx.http, "✅ Goodbye message text updated.").await?;
        }
        "toggle" => {
            let enabled = db::toggle_goodbye(pool, guild_id.get() as i64).await?;
            let status = if enabled { "enabled" } else { "disabled" };
            msg.reply(&ctx.http, format!("✅ Goodbye messages are now **{}**.", status)).await?;
        }
        _ => {
            msg.reply(&ctx.http, "ℹ️ Usage: `$goodbye channel #ch` | `$goodbye text <msg>` | `$goodbye toggle` | `$goodbyetest`").await?;
        }
    }

    Ok(())
}

#[command]
#[description = "Send a test goodbye message embed."]
pub async fn goodbyetest(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let bot_data = data.get::<BotData>().unwrap();
    let cfg = &bot_data.config;

    if !check_user_mod(ctx, msg, cfg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }

    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return Ok(()),
    };

    let goodbye_cfg = db::get_goodbye_config(&bot_data.db, guild_id.get() as i64).await?;
    let goodbye_cfg = match goodbye_cfg {
        Some(c) => c,
        None => {
            msg.reply(&ctx.http, "❌ Goodbye channel is not set yet. Use `$goodbye channel #channel` first.").await?;
            return Ok(());
        }
    };

    let target_ch = if goodbye_cfg.channel_id > 0 { ChannelId::new(goodbye_cfg.channel_id as u64) } else { msg.channel_id };
    send_goodbye_embed(ctx, guild_id, &msg.author, target_ch, &goodbye_cfg).await;
    msg.reply(&ctx.http, format!("✅ Sent test goodbye message to <#{}>.", target_ch)).await?;
    Ok(())
}
```

- [ ] **Step 3: Export in `src/handlers/mod.rs` and `src/handlers/commands/mod.rs`**

- [ ] **Step 4: Verify build with `cargo check`**

Run: `cargo check`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/handlers/goodbye.rs src/handlers/commands/goodbye.rs src/handlers/mod.rs src/handlers/commands/mod.rs
git commit -m "feat(goodbye): add member leave handler and $goodbye commands"
```

---

### Task 7: Wire Handlers & Framework Groups in `src/main.rs` & End-to-End Verification

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Register event handlers and groups in `src/main.rs`**

In `EventHandler`:
```rust
    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        reaction_roles::handle_reaction_add(&ctx, &reaction).await;
    }

    async fn reaction_remove(&self, ctx: Context, reaction: Reaction) {
        reaction_roles::handle_reaction_remove(&ctx, &reaction).await;
    }

    async fn guild_member_removal(&self, ctx: Context, guild_id: GuildId, user: User, member_data: Option<Member>) {
        logging::handle_member_leave(&ctx, guild_id, &user, &member_data).await;
        goodbye::handle_member_leave(&ctx, guild_id, &user, &member_data).await;
    }
```

In `StandardFramework`:
```rust
    let framework = StandardFramework::new()
        .help(&MY_HELP)
        .group(&MODCMDS_GROUP)
        .group(&WARNCMDS_GROUP)
        .group(&WELCOMECMDS_GROUP)
        .group(&GOODBYECMDS_GROUP)
        .group(&RRCMDS_GROUP)
        .group(&UTILITYCMDS_GROUP)
        .group(&EMBEDCMDS_GROUP)
        .group(&RAID_GROUP);
```

- [ ] **Step 2: Run `cargo test` and `cargo check`**

Run: `cargo test`
Expected: PASS with zero errors

- [ ] **Step 3: Final Commit**

```bash
git add src/main.rs
git commit -m "feat(main): wire Carl-bot suite handlers and command groups"
```
