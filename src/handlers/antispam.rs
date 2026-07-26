use serenity::{
    all::*,
    prelude::*,
    framework::standard::{macros::command, CommandResult},
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Instant,
};
use dashmap::DashMap;
use regex::Regex;
use std::sync::OnceLock;
use crate::{BotData, utils::{log_action, mod_embed}};

// ── State shared via TypeMap ──────────────────────────────────────────────────

/// Per-user message timestamps for spam detection.
pub struct SpamTracker(pub Arc<DashMap<u64, VecDeque<Instant>>>);
impl TypeMapKey for SpamTracker { type Value = SpamTracker; }

/// Join timestamps + raid-active flag.
pub struct RaidState(pub Arc<Mutex<(VecDeque<Instant>, bool)>>);
impl TypeMapKey for RaidState { type Value = RaidState; }

static INVITE_RE: OnceLock<Regex> = OnceLock::new();

fn invite_re() -> &'static Regex {
    INVITE_RE.get_or_init(|| {
        Regex::new(r"(?i)(discord\.gg|discord\.com/invite)/[a-zA-Z0-9]+").unwrap()
    })
}

// ── on_message ────────────────────────────────────────────────────────────────

pub async fn handle_message(ctx: &Context, msg: &Message) {
    if msg.author.bot || msg.guild_id.is_none() {
        return;
    }

    // Extract config + immune check before any await (CacheRef is !Send)
    let (spam_limit, spam_window, anti_invite, mod_log, is_immune) = {
        let data = ctx.data.read().await;
        let bot_data = data.get::<BotData>().unwrap();
        let cfg = &bot_data.config;

        let is_immune = ctx.cache
            .guild(msg.guild_id.unwrap())
            .and_then(|g| g.members.get(&msg.author.id).map(|m| cfg.is_immune(m)))
            .unwrap_or(false);

        (cfg.spam_msg_limit, cfg.spam_window_secs, cfg.anti_invite, cfg.mod_log_channel, is_immune)
    };

    if is_immune {
        return;
    }

    check_spam(ctx, msg, spam_limit, spam_window, mod_log).await;
    if anti_invite {
        check_invite(ctx, msg, mod_log).await;
    }
}

async fn check_spam(
    ctx:             &Context,
    msg:             &Message,
    spam_msg_limit:  usize,
    spam_window_secs: u64,
    mod_log_channel: u64,
) {
    let uid  = msg.author.id.get();
    let now  = Instant::now();
    let window = std::time::Duration::from_secs(spam_window_secs);

    let fire = {
        let data = ctx.data.read().await;
        let tracker = data.get::<SpamTracker>().unwrap();
        let mut dq = tracker.0.entry(uid).or_default();
        dq.push_back(now);
        while dq.front().map(|t| now.duration_since(*t) > window).unwrap_or(false) {
            dq.pop_front();
        }
        if dq.len() >= spam_msg_limit {
            dq.clear();
            true
        } else {
            false
        }
    }; // lock released

    if !fire {
        return;
    }

    tracing::info!("antispam: spam detected from {uid}");

    // Timeout 10 minutes
    let until = chrono::Utc::now() + chrono::Duration::minutes(10);
    let until_str = until.to_rfc3339();
    if let Some(guild_id) = msg.guild_id {
        let _ = guild_id
            .edit_member(&ctx.http, msg.author.id, EditMember::new().disable_communication_until(until_str))
            .await;
    }

    let embed = mod_embed(
        "🚫 Spam Detected",
        Color::ORANGE,
        &[
            ("User",    &format!("<@{uid}> (`{uid}`)")),
            ("Action",  "Timeout 10 minutes"),
            ("Channel", &format!("<#{}>", msg.channel_id)),
        ],
    );
    log_action(&ctx.http, mod_log_channel, embed).await;
}

async fn check_invite(ctx: &Context, msg: &Message, mod_log_channel: u64) {
    if !invite_re().is_match(&msg.content) {
        return;
    }

    let _ = msg.delete(&ctx.http).await;
    let _ = msg.channel_id
        .say(&ctx.http, format!("{} Invite links are not allowed.", msg.author.mention()))
        .await;

    let embed = mod_embed(
        "🔗 Invite Link Blocked",
        Color::GOLD,
        &[
            ("User",    &format!("<@{}>", msg.author.id)),
            ("Channel", &format!("<#{}>", msg.channel_id)),
        ],
    );
    log_action(&ctx.http, mod_log_channel, embed).await;
}

// ── on_member_join (raid detection) ──────────────────────────────────────────

pub async fn handle_member_join(ctx: &Context, member: &Member) {
    let (raid_join_limit, raid_window_secs, mod_log_channel, should_activate) = {
        let data = ctx.data.read().await;
        let bot_data = data.get::<BotData>().unwrap();
        let cfg = &bot_data.config;
        let raid_state = data.get::<RaidState>().unwrap();

        let now    = Instant::now();
        let window = std::time::Duration::from_secs(cfg.raid_window_secs);

        let should_activate = {
            let mut state = raid_state.0.lock().unwrap();
            let (joins, active) = &mut *state;
            joins.push_back(now);
            while joins.front().map(|t| now.duration_since(*t) > window).unwrap_or(false) {
                joins.pop_front();
            }
            joins.len() >= cfg.raid_join_limit && !*active
        };

        (cfg.raid_join_limit, cfg.raid_window_secs, cfg.mod_log_channel, should_activate)
    };

    if should_activate {
        enable_raid_mode(ctx, member.guild_id, raid_join_limit, raid_window_secs, mod_log_channel).await;

        let http_clone = Arc::clone(&ctx.http);
        let data_clone = Arc::clone(&ctx.data);
        let guild_id   = member.guild_id;

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(600)).await;
            disable_raid_mode_inner(&http_clone, &data_clone, guild_id).await;
        });
    }
}

async fn enable_raid_mode(
    ctx:              &Context,
    guild_id:         GuildId,
    raid_join_limit:  usize,
    raid_window_secs: u64,
    mod_log_channel:  u64,
) {
    {
        let data = ctx.data.read().await;
        let raid_state = data.get::<RaidState>().unwrap();
        raid_state.0.lock().unwrap().1 = true;
    }

    tracing::warn!("antispam: RAID MODE ENABLED in {}", guild_id);

    let _ = guild_id
        .edit(&ctx.http, EditGuild::new().verification_level(VerificationLevel::Higher))
        .await;

    let embed = mod_embed(
        "🚨 RAID MODE ACTIVATED",
        Color::DARK_RED,
        &[
            ("Reason", &format!("{raid_join_limit} joins in {raid_window_secs}s")),
            ("Action", "Verification level set to HIGHEST"),
            ("Note",   "Use !raidoff to disable"),
        ],
    );
    log_action(&ctx.http, mod_log_channel, embed).await;
}

pub async fn disable_raid_mode_inner(
    http: &Arc<Http>,
    data: &Arc<tokio::sync::RwLock<TypeMap>>,
    guild_id: GuildId,
) {
    let mod_log_channel = {
        let d = data.read().await;
        d.get::<BotData>().unwrap().config.mod_log_channel
    };

    {
        let d = data.read().await;
        d.get::<RaidState>().unwrap().0.lock().unwrap().1 = false;
    }

    let _ = guild_id
        .edit(http, EditGuild::new().verification_level(VerificationLevel::Medium))
        .await;

    tracing::info!("antispam: raid mode disabled");

    let embed = mod_embed(
        "✅ Raid Mode Disabled",
        Color::DARK_GREEN,
        &[("Action", "Verification restored to medium")],
    );
    log_action(http, mod_log_channel, embed).await;
}

// ── !raidoff command ──────────────────────────────────────────────────────────

#[command]
#[description = "Manually disable raid mode."]
pub async fn raidoff(ctx: &Context, msg: &Message) -> CommandResult {
    let (is_mod, guild_id) = {
        let data = ctx.data.read().await;
        let cfg = &data.get::<BotData>().unwrap().config;
        let is_mod = ctx.cache
            .guild(msg.guild_id.unwrap_or_default())
            .and_then(|g| g.members.get(&msg.author.id).map(|m| cfg.is_mod(m)))
            .unwrap_or(false);
        (is_mod, msg.guild_id)
    };

    if !is_mod {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }
    let guild_id = match guild_id {
        Some(g) => g,
        None => return Ok(()),
    };

    let http_clone = Arc::clone(&ctx.http);
    let data_clone = Arc::clone(&ctx.data);
    disable_raid_mode_inner(&http_clone, &data_clone, guild_id).await;
    msg.reply(&ctx.http, "✅ Raid mode disabled.").await?;
    Ok(())
}
