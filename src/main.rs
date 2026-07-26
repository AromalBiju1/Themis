/*!
ThemisBot — Rust edition
========================
Security-first Discord moderation bot.

Features:
  - honeypot   : softban + 24hr message wipe on honeypot trigger
  - antispam   : rate-limit spam, anti-invite, anti-raid
  - modcmds    : ban, kick, softban, mute, unmute, purge, slowmode, unban
  - warns      : persistent warn system with escalation (mute → kick → ban)
  - logging    : join/leave/edit/delete/ban events
  - autorole   : assign role on member join
  - health     : HTTP /health endpoint

Required env vars:
  DISCORD_TOKEN, GUILD_ID, MOD_LOG_CHANNEL_ID

Discord privileged intents required:
  - MESSAGE CONTENT INTENT
  - SERVER MEMBERS INTENT
*/

mod config;
mod db;
mod utils;
mod handlers;

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use axum::{routing::get, Router};
use dashmap::DashMap;
use serenity::{
    all::*,
    async_trait,
    framework::standard::{
        macros::group, Configuration, StandardFramework,
    },
    prelude::*,
};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use tracing::info;

use config::Config;
use handlers::{
    antispam::{self, RaidState, SpamTracker, RAIDOFF_COMMAND},
    autorole, honeypot, logging,
    commands::{
        modcmds::MODCMDS_GROUP,
        warns::WARNCMDS_GROUP,
    },
};

// ── Raidoff command group ─────────────────────────────────────────────────────

#[group]
#[commands(raidoff)]
struct Raid;

// ── Help command ──────────────────────────────────────────────────────────────
use std::collections::HashSet;
use serenity::framework::standard::{help_commands, macros::help, Args, CommandGroup, CommandResult, HelpOptions};

#[help]
#[command_not_found_text = "Could not find command: `{}`."]
#[max_levenshtein_distance(3)]
async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

// ── Shared data stored in serenity's TypeMap ─────────────────────────────────

pub struct BotData {
    pub config: Config,
    pub db:     SqlitePool,
}

impl TypeMapKey for BotData {
    type Value = BotData;
}

// ── Event handler ─────────────────────────────────────────────────────────────

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("Logged in as {} (ID: {})", ready.user.name, ready.user.id);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        honeypot::handle_message(&ctx, &msg).await;
        antispam::handle_message(&ctx, &msg).await;
    }

    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        logging::handle_member_join(&ctx, &member).await;
        autorole::handle_member_join(&ctx, &member).await;
        antispam::handle_member_join(&ctx, &member).await;
    }

    async fn guild_member_removal(
        &self,
        ctx:         Context,
        guild_id:    GuildId,
        user:        User,
        member_data: Option<Member>,
    ) {
        logging::handle_member_leave(&ctx, guild_id, &user, &member_data).await;
    }

    async fn message_update(
        &self,
        ctx:   Context,
        old:   Option<Message>,
        new:   Option<Message>,
        event: MessageUpdateEvent,
    ) {
        logging::handle_message_edit(&ctx, &old, &new, &event).await;
    }

    async fn message_delete(
        &self,
        ctx:        Context,
        channel_id: ChannelId,
        deleted_id: MessageId,
        guild_id:   Option<GuildId>,
    ) {
        logging::handle_message_delete(&ctx, channel_id, deleted_id, guild_id).await;
    }

    async fn guild_ban_addition(&self, ctx: Context, guild_id: GuildId, user: User) {
        logging::handle_ban(&ctx, guild_id, &user).await;
    }

    async fn guild_ban_removal(&self, ctx: Context, guild_id: GuildId, user: User) {
        logging::handle_unban(&ctx, guild_id, &user).await;
    }
}

// ── Health server ─────────────────────────────────────────────────────────────

async fn health_handler() -> &'static str {
    "OK"
}

async fn start_health_server(port: u16) {
    let app = Router::new().route("/health", get(health_handler));
    let addr = format!("0.0.0.0:{port}");
    info!("Health server on :{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ── main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "themisbot=info,serenity=warn".into()),
        )
        .init();

    // Load config from .env / environment
    let cfg = Config::load()?;

    // Open SQLite connection pool — create_if_missing so the file is created on first run
    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(&cfg.db_path)
            .create_if_missing(true),
    )
    .await?;
    db::init_db(&pool).await?;
    info!("Database ready at {}", cfg.db_path);

    // Set up the serenity framework with prefix commands
    let framework = StandardFramework::new()
        .help(&MY_HELP)
        .group(&MODCMDS_GROUP)
        .group(&WARNCMDS_GROUP)
        .group(&RAID_GROUP);

    framework.configure(Configuration::new().prefix("!"));

    // Build intents
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MODERATION
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    // Build client
    let token = cfg.discord_token.clone();
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await?;

    // Insert shared data into TypeMap
    {
        let mut data = client.data.write().await;
        data.insert::<BotData>(BotData { config: cfg, db: pool });
        data.insert::<SpamTracker>(SpamTracker(Arc::new(DashMap::new())));
        data.insert::<RaidState>(RaidState(Arc::new(Mutex::new((VecDeque::new(), false)))));
    }

    // Spawn health server
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8085);
    tokio::spawn(start_health_server(port));

    // Start bot (auto-sharded)
    info!("Starting ThemisBot…");
    client.start_autosharded().await?;
    Ok(())
}
