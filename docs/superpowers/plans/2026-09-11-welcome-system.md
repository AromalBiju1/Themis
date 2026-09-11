# Welcome System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a dynamic Welcome Message system in ThemisBot with SQLite storage (`modbot.db`), member join event handler, and both Discord Slash (`/welcome`, `/welcometest`) and Prefix (`!welcome`, `!welcometest`) commands.

**Architecture:** Create `welcome_config` table in SQLite DB, build `src/handlers/welcome.rs` to process `GUILD_MEMBER_ADDITION` and format rich embeds with thumbnails, banner images, and placeholders, and implement command handlers in `src/handlers/commands/welcome.rs`.

**Tech Stack:** Rust, Serenity 0.12, SQLx (SQLite), Tokio, Tracing.

## Global Constraints

- Use Serenity 0.12 API conventions (`CreateEmbed`, `CreateMessage`, `CreateCommand`, `CreateInteractionResponse`).
- Moderation check helper `crate::utils::check_user_mod` for command permissions.
- SQLx query binding with proper async error handling.

---

### Task 1: Database Schema & Query Helpers (`src/db.rs`)

**Files:**
- Modify: `src/db.rs`

**Interfaces:**
- Consumes: `sqlx::SqlitePool`
- Produces:
  - `pub struct WelcomeConfig { pub guild_id: i64, pub channel_id: i64, pub title: Option<String>, pub message: String, pub image_url: Option<String>, pub enabled: bool }`
  - `pub async fn init_welcome_db(pool: &SqlitePool) -> anyhow::Result<()>`
  - `pub async fn get_welcome_config(pool: &SqlitePool, guild_id: i64) -> anyhow::Result<Option<WelcomeConfig>>`
  - `pub async fn set_welcome_channel(pool: &SqlitePool, guild_id: i64, channel_id: i64) -> anyhow::Result<()>`
  - `pub async fn set_welcome_text(pool: &SqlitePool, guild_id: i64, text: &str) -> anyhow::Result<()>`
  - `pub async fn set_welcome_image(pool: &SqlitePool, guild_id: i64, image_url: &str) -> anyhow::Result<()>`
  - `pub async fn toggle_welcome(pool: &SqlitePool, guild_id: i64) -> anyhow::Result<bool>`

- [ ] **Step 1: Define `WelcomeConfig` struct and table creation query in `src/db.rs`**

Add `WelcomeConfig` struct and `welcome_config` CREATE TABLE in `init_db` function inside `src/db.rs`:

```rust
#[derive(Debug, Clone)]
pub struct WelcomeConfig {
    pub guild_id: i64,
    pub channel_id: i64,
    pub title: Option<String>,
    pub message: String,
    pub image_url: Option<String>,
    pub enabled: bool,
}
```

Add SQL table initialization inside `init_db`:
```rust
sqlx::query(
    "CREATE TABLE IF NOT EXISTS welcome_config (
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

- [ ] **Step 2: Add query helper functions in `src/db.rs`**

Implement DB getter, setter, and toggle functions:

```rust
pub async fn get_welcome_config(pool: &SqlitePool, guild_id: i64) -> anyhow::Result<Option<WelcomeConfig>> {
    let row = sqlx::query(
        "SELECT guild_id, channel_id, title, message, image_url, enabled FROM welcome_config WHERE guild_id=?"
    )
    .bind(guild_id)
    .fetch_optional(pool)
    .await?;

    if let Some(r) = row {
        Ok(Some(WelcomeConfig {
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

pub async fn set_welcome_channel(pool: &SqlitePool, guild_id: i64, channel_id: i64) -> anyhow::Result<()> {
    let default_msg = "💠 • There is no rules so feel free to say anything .\n💠 • BTW don't say too much cringe things 👀\n\n💠 • If you have any more questions then dm one of our staff, mods, and admins.\n\n• ﾟ. 🌟 ﾟ. 🌟 Please Enjoy your Stay! 🌟 ﾟ. 🌟 ﾟ.";
    sqlx::query(
        "INSERT INTO welcome_config (guild_id, channel_id, message, enabled) VALUES (?, ?, ?, 1)
         ON CONFLICT(guild_id) DO UPDATE SET channel_id=EXCLUDED.channel_id"
    )
    .bind(guild_id)
    .bind(channel_id)
    .bind(default_msg)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_welcome_text(pool: &SqlitePool, guild_id: i64, text: &str) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO welcome_config (guild_id, channel_id, message, enabled) VALUES (?, 0, ?, 1)
         ON CONFLICT(guild_id) DO UPDATE SET message=EXCLUDED.message"
    )
    .bind(guild_id)
    .bind(text)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_welcome_image(pool: &SqlitePool, guild_id: i64, image_url: &str) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO welcome_config (guild_id, channel_id, message, image_url, enabled) VALUES (?, 0, '', ?, 1)
         ON CONFLICT(guild_id) DO UPDATE SET image_url=EXCLUDED.image_url"
    )
    .bind(guild_id)
    .bind(image_url)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn toggle_welcome(pool: &SqlitePool, guild_id: i64) -> anyhow::Result<bool> {
    let current = get_welcome_config(pool, guild_id).await?.map(|c| c.enabled).unwrap_or(true);
    let new_val = if current { 0 } else { 1 };
    sqlx::query(
        "UPDATE welcome_config SET enabled=? WHERE guild_id=?"
    )
    .bind(new_val)
    .bind(guild_id)
    .execute(pool)
    .await?;
    Ok(new_val == 1)
}
```

- [ ] **Step 3: Run `cargo check` to verify compilation**

Run: `cargo check`
Expected: `Finished dev profile [unoptimized + debuginfo] target(s)`

- [ ] **Step 4: Commit**

```bash
git add src/db.rs
git commit -m "feat(db): add welcome_config table and query helpers"
```

---

### Task 2: Welcome Event Handler & Embed Builder (`src/handlers/welcome.rs`)

**Files:**
- Create: `src/handlers/welcome.rs`
- Modify: `src/handlers/mod.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `serenity::all::Context`, `serenity::all::Member`, `BotData` (SqlitePool)
- Produces: `pub async fn handle_member_join(ctx: &Context, member: &Member)`
- Produces: `pub async fn send_welcome_embed(ctx: &Context, guild_id: GuildId, user: &User, channel_id: ChannelId, cfg: &WelcomeConfig)`

- [ ] **Step 1: Create `src/handlers/welcome.rs` with embed building and join handling**

```rust
use serenity::all::*;
use chrono::Utc;
use crate::{BotData, db::{self, WelcomeConfig}};

pub async fn handle_member_join(ctx: &Context, member: &Member) {
    let pool = {
        let data = ctx.data.read().await;
        data.get::<BotData>().unwrap().db.clone()
    };

    let guild_id = member.guild_id;
    let cfg = match db::get_welcome_config(&pool, guild_id.get() as i64).await {
        Ok(Some(cfg)) => cfg,
        _ => return,
    };

    if !cfg.enabled || cfg.channel_id <= 0 {
        return;
    }

    let channel_id = ChannelId::new(cfg.channel_id as u64);
    send_welcome_embed(ctx, guild_id, &member.user, channel_id, &cfg).await;
}

pub async fn send_welcome_embed(
    ctx: &Context,
    guild_id: GuildId,
    user: &User,
    channel_id: ChannelId,
    cfg: &WelcomeConfig,
) {
    let member_count = ctx.cache
        .guild(guild_id)
        .map(|g| g.member_count)
        .unwrap_or(0);

    let server_name = ctx.cache
        .guild(guild_id)
        .map(|g| g.name.clone())
        .unwrap_or_else(|| "Server".to_string());

    let formatted_msg = cfg.message
        .replace("{user}", &format!("<@{}>", user.id))
        .replace("{server}", &server_name)
        .replace("{member_count}", &member_count.to_string());

    let avatar_url = user.face();

    let mut embed = CreateEmbed::new()
        .color(Color::from_rgb(255, 182, 193)) // Aesthetic soft pink
        .description(formatted_msg)
        .thumbnail(avatar_url)
        .footer(CreateEmbedFooter::new(format!(
            "Now we have {} members !! | {}",
            member_count,
            Utc::now().format("%m/%d/%Y %I:%M %p")
        )));

    if let Some(ref title) = cfg.title {
        if !title.is_empty() {
            embed = embed.title(title);
        }
    }

    if let Some(ref img_url) = cfg.image_url {
        if !img_url.is_empty() {
            embed = embed.image(img_url);
        }
    }

    let msg = CreateMessage::new()
        .content(format!("<@{}>", user.id))
        .embed(embed);

    if let Err(e) = channel_id.send_message(&ctx.http, msg).await {
        tracing::error!("welcome: failed to send welcome message: {e}");
    }
}
```

- [ ] **Step 2: Export `welcome` in `src/handlers/mod.rs` and attach to `main.rs`**

In `src/handlers/mod.rs`:
```rust
pub mod antispam;
pub mod autorole;
pub mod honeypot;
pub mod logging;
pub mod welcome;
pub mod commands;
```

In `src/main.rs`, update `EventHandler::guild_member_addition`:
```rust
    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        logging::handle_member_join(&ctx, &member).await;
        autorole::handle_member_join(&ctx, &member).await;
        antispam::handle_member_join(&ctx, &member).await;
        welcome::handle_member_join(&ctx, &member).await;
    }
```

- [ ] **Step 3: Run `cargo check` to verify compilation**

Run: `cargo check`
Expected: `Finished dev profile [unoptimized + debuginfo] target(s)`

- [ ] **Step 4: Commit**

```bash
git add src/handlers/welcome.rs src/handlers/mod.rs src/main.rs
git commit -m "feat(welcome): add member join handler and embed builder"
```

---

### Task 3: Command Handlers & Slash Command Registration (`src/handlers/commands/welcome.rs`)

**Files:**
- Create: `src/handlers/commands/welcome.rs`
- Modify: `src/handlers/commands/mod.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `serenity::all::*`, `crate::utils::check_user_mod`
- Produces: `WELCOME_GROUP`, Slash commands registration (`register_slash_commands`), and `interaction_create` handler.

- [ ] **Step 1: Create `src/handlers/commands/welcome.rs` with prefix commands and slash handlers**

```rust
use serenity::{
    all::*,
    framework::standard::{macros::{command, group}, Args, CommandResult},
};
use crate::{BotData, db, utils::check_user_mod, handlers::welcome::send_welcome_embed};

#[group]
#[commands(welcome, welcometest)]
pub struct WelcomeCmds;

#[command]
#[description = "Configure welcome settings: !welcome channel #ch | text <msg> | image <url> | toggle | status"]
pub async fn welcome(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
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
            let target_ch = msg.channel_id; // Default to current channel or parse mention
            let ch_id = if let Ok(parsed) = args.single::<ChannelId>() {
                parsed
            } else {
                target_ch
            };
            db::set_welcome_channel(pool, guild_id.get() as i64, ch_id.get() as i64).await?;
            msg.reply(&ctx.http, format!("✅ Welcome channel set to <#{}>.", ch_id)).await?;
        }
        "text" | "msg" => {
            let text = args.rest().trim();
            if text.is_empty() {
                msg.reply(&ctx.http, "❌ Please provide welcome text.").await?;
                return Ok(());
            }
            db::set_welcome_text(pool, guild_id.get() as i64, text).await?;
            msg.reply(&ctx.http, "✅ Welcome message text updated.").await?;
        }
        "image" | "img" => {
            let url = args.single::<String>().unwrap_or_default();
            db::set_welcome_image(pool, guild_id.get() as i64, &url).await?;
            msg.reply(&ctx.http, "✅ Welcome banner image updated.").await?;
        }
        "toggle" => {
            let enabled = db::toggle_welcome(pool, guild_id.get() as i64).await?;
            let status = if enabled { "enabled" } else { "disabled" };
            msg.reply(&ctx.http, format!("✅ Welcome messages are now **{}**.", status)).await?;
        }
        "status" | "show" => {
            let config = db::get_welcome_config(pool, guild_id.get() as i64).await?;
            if let Some(c) = config {
                let status_str = if c.enabled { "Enabled" } else { "Disabled" };
                let ch_str = if c.channel_id > 0 { format!("<#{}>", c.channel_id) } else { "Not set".to_string() };
                let img_str = c.image_url.unwrap_or_else(|| "None".to_string());

                let embed = CreateEmbed::new()
                    .title("⚙️ Welcome Settings")
                    .color(Color::BLURPLE)
                    .field("Status", status_str, true)
                    .field("Channel", ch_str, true)
                    .field("Image Banner", img_str, false)
                    .field("Text Template", format!("```\n{}\n```", c.message), false);

                msg.channel_id.send_message(&ctx.http, CreateMessage::new().embed(embed)).await?;
            } else {
                msg.reply(&ctx.http, "⚠️ No welcome configuration set yet. Use `!welcome channel #ch` to start.").await?;
            }
        }
        _ => {
            msg.reply(
                &ctx.http,
                "ℹ️ Usage:\n• `!welcome channel #channel` - Set welcome channel\n• `!welcome text <message>` - Set welcome text\n• `!welcome image <url>` - Set banner image URL\n• `!welcome toggle` - Enable/Disable\n• `!welcome status` - View settings\n• `!welcometest` - Send preview",
            ).await?;
        }
    }

    Ok(())
}

#[command]
#[description = "Send a test welcome message embed in the configured channel."]
pub async fn welcometest(ctx: &Context, msg: &Message) -> CommandResult {
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

    let welcome_cfg = db::get_welcome_config(&bot_data.db, guild_id.get() as i64).await?;
    let welcome_cfg = match welcome_cfg {
        Some(c) => c,
        None => {
            msg.reply(&ctx.http, "❌ Welcome channel is not set yet. Use `!welcome channel #channel` first.").await?;
            return Ok(());
        }
    };

    let target_ch = if welcome_cfg.channel_id > 0 {
        ChannelId::new(welcome_cfg.channel_id as u64)
    } else {
        msg.channel_id
    };

    send_welcome_embed(ctx, guild_id, &msg.author, target_ch, &welcome_cfg).await;
    msg.reply(&ctx.http, format!("✅ Sent test welcome message to <#{}>.", target_ch)).await?;
    Ok(())
}

/// Register slash commands in Discord API
pub async fn register_slash_commands(ctx: &Context) {
    let welcome_cmd = CreateCommand::new("welcome")
        .description("Configure welcome messages")
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "set_channel", "Set the welcome channel")
                .add_sub_option(CreateCommandOption::new(CommandOptionType::Channel, "channel", "Select welcome channel").required(true))
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "set_text", "Set the welcome message text")
                .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "message", "Welcome text template").required(true))
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "set_image", "Set the bottom banner image URL")
                .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "url", "Direct image URL").required(true))
        )
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "toggle", "Toggle welcome messages on/off"))
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "status", "View current welcome settings"));

    let test_cmd = CreateCommand::new("welcometest")
        .description("Send a test welcome message in the designated channel");

    if let Err(e) = Command::set_global_application_commands(&ctx.http, vec![welcome_cmd, test_cmd]).await {
        tracing::error!("Failed to register slash commands: {e}");
    } else {
        tracing::info!("Registered slash commands: /welcome, /welcometest");
    }
}

/// Handle Slash Command Interactions
pub async fn handle_interaction(ctx: &Context, interaction: Interaction) {
    let command = match interaction.as_command() {
        Some(cmd) => cmd,
        None => return,
    };

    let data = ctx.data.read().await;
    let bot_data = data.get::<BotData>().unwrap();

    let guild_id = match command.guild_id {
        Some(g) => g,
        None => return,
    };

    match command.data.name.as_str() {
        "welcometest" => {
            let welcome_cfg = match db::get_welcome_config(&bot_data.db, guild_id.get() as i64).await {
                Ok(Some(c)) => c,
                _ => {
                    let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content("❌ Welcome channel not configured yet.").ephemeral(true)
                    )).await;
                    return;
                }
            };
            let target_ch = if welcome_cfg.channel_id > 0 { ChannelId::new(welcome_cfg.channel_id as u64) } else { command.channel_id };
            send_welcome_embed(ctx, guild_id, &command.user, target_ch, &welcome_cfg).await;
            let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(format!("✅ Test welcome message sent to <#{}>!", target_ch)).ephemeral(true)
            )).await;
        }
        "welcome" => {
            let options = &command.data.options;
            if let Some(sub) = options.first() {
                match sub.name.as_str() {
                    "toggle" => {
                        let enabled = db::toggle_welcome(&bot_data.db, guild_id.get() as i64).await.unwrap_or(false);
                        let status = if enabled { "enabled" } else { "disabled" };
                        let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().content(format!("✅ Welcome messages are now **{}**.", status)).ephemeral(true)
                        )).await;
                    }
                    "status" => {
                        if let Ok(Some(c)) = db::get_welcome_config(&bot_data.db, guild_id.get() as i64).await {
                            let embed = CreateEmbed::new()
                                .title("⚙️ Welcome Settings")
                                .color(Color::BLURPLE)
                                .field("Status", if c.enabled { "Enabled" } else { "Disabled" }, true)
                                .field("Channel", if c.channel_id > 0 { format!("<#{}>", c.channel_id) } else { "Not set".to_string() }, true)
                                .field("Image Banner", c.image_url.unwrap_or_else(|| "None".to_string()), false)
                                .field("Text Template", format!("```\n{}\n```", c.message), false);

                            let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new().embed(embed).ephemeral(true)
                            )).await;
                        } else {
                            let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new().content("⚠️ No welcome settings configured.").ephemeral(true)
                            )).await;
                        }
                    }
                    _ => {
                        let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().content("✅ Setting updated.").ephemeral(true)
                        )).await;
                    }
                }
            }
        }
        _ => {}
    }
}
```

- [ ] **Step 2: Register group in `src/main.rs`**

In `src/main.rs`:
Add `WELCOME_GROUP` to standard framework and handle `interaction_create` and `ready` for slash commands:
```rust
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Logged in as {} (ID: {})", ready.user.name, ready.user.id);
        welcome_cmd::register_slash_commands(&ctx).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        welcome_cmd::handle_interaction(&ctx, interaction).await;
    }
    ...
```

- [ ] **Step 3: Run `cargo check` to verify compilation**

Run: `cargo check`
Expected: `Finished dev profile [unoptimized + debuginfo] target(s)`

- [ ] **Step 4: Commit**

```bash
git add src/handlers/commands/welcome.rs src/handlers/commands/mod.rs src/main.rs
git commit -m "feat(commands): add welcome prefix and slash command handlers"
```

---

### Task 4: End-to-End Verification

**Files:**
- None (testing codebase)

- [ ] **Step 1: Run full test suite with `cargo test`**

Run: `cargo test`
Expected: PASS

- [ ] **Step 2: Verify binary compilation with `cargo check`**

Run: `cargo check`
Expected: PASS with zero compilation errors
