use serenity::all::*;
use std::{collections::HashSet, sync::Mutex};
use crate::{BotData, utils::{log_action, mod_embed}};

/// Tracks which member IDs are already being processed to avoid double-bans.
static PROCESSING: std::sync::LazyLock<Mutex<HashSet<u64>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashSet::new()));

/// Called from the top-level EventHandler on every message.
pub async fn handle_message(ctx: &Context, msg: &Message) {
    let (honeypot_channel, mod_log_channel, is_immune) = {
        let data = ctx.data.read().await;
        let bot_data = data.get::<BotData>().unwrap();
        let cfg = &bot_data.config;

        if msg.channel_id.get() != cfg.honeypot_channel {
            return;
        }
        if msg.author.id == ctx.cache.current_user().id {
            return;
        }

        // Extract guild/member data while the CacheRef is alive, then drop it
        let is_immune = ctx.cache
            .guild(msg.guild_id.unwrap_or_default())
            .and_then(|g| g.members.get(&msg.author.id).map(|m| cfg.is_immune(m)))
            .unwrap_or(false);

        (cfg.honeypot_channel, cfg.mod_log_channel, is_immune)
    }; // ← data guard + any CacheRef dropped here

    if msg.guild_id.is_none() {
        return;
    }
    if is_immune {
        return;
    }

    let uid = msg.author.id.get();
    let guild_id = msg.guild_id.unwrap();

    // Dedup guard
    {
        let mut set = PROCESSING.lock().unwrap();
        if set.contains(&uid) {
            return;
        }
        set.insert(uid);
    }

    // Delete the triggering message
    if let Err(e) = msg.delete(&ctx.http).await {
        tracing::warn!("honeypot: couldn't delete message: {e}");
    }

    // Softban: ban (wipes 24h of messages) then immediately unban
    let ban_result = guild_id
        .ban_with_reason(&ctx.http, msg.author.id, 1, "Honeypot triggered")
        .await;

    match ban_result {
        Ok(_) => {
            tracing::info!("honeypot: banned {uid}");
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            if let Err(e) = guild_id.unban(&ctx.http, msg.author.id).await {
                tracing::error!("honeypot: unban failed for {uid}: {e}");
            } else {
                tracing::info!("honeypot: unbanned {uid}");
            }

            let embed = mod_embed(
                "🍯 Honeypot Triggered",
                Color::RED,
                &[
                    ("User", &format!("<@{uid}> (`{uid}`)")),
                    ("Action", "Softban — 24hr messages wiped server-wide"),
                ],
            );
            log_action(&ctx.http, mod_log_channel, embed).await;
        }
        Err(e) => {
            tracing::error!("honeypot: ban failed for {uid}: {e}");
        }
    }

    PROCESSING.lock().unwrap().remove(&uid);
}
