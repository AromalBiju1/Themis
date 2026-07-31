use serenity::{
    all::*,
    builder::{CreateEmbed, CreateMessage},
};
use chrono::Utc;

/// Build a standardised mod-action embed.
pub fn mod_embed(title: &str, color: Color, fields: &[(&str, &str)]) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title(title)
        .color(color)
        .timestamp(Utc::now())
        .footer(CreateEmbedFooter::new("Themis"));

    for (name, value) in fields {
        embed = embed.field(*name, *value, false);
    }
    embed
}

/// Send an embed to the mod-log channel.
pub async fn log_action(
    http:            &Http,
    mod_log_channel: u64,
    embed:           CreateEmbed,
) {
    if mod_log_channel == 0 {
        return;
    }
    let ch = ChannelId::new(mod_log_channel);
    let msg = CreateMessage::new().embed(embed);
    if let Err(e) = ch.send_message(http, msg).await {
        tracing::error!("log_action failed: {e}");
    }
}

/// Robustly parse a user ID from a raw string or a Discord mention (`<@id>` or `<@!id>`).
pub fn parse_target(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Ok(id) = s.parse::<u64>() {
        return Some(id);
    }
    if s.starts_with("<@") && s.ends_with('>') {
        let clean = s.trim_start_matches("<@").trim_start_matches('!').trim_end_matches('>');
        if let Ok(id) = clean.parse::<u64>() {
            return Some(id);
        }
    }
    None
}

/// Helper to parse a target ID from command arguments.
pub fn parse_target_from_args(args: &mut serenity::framework::standard::Args) -> Result<u64, ()> {
    if let Ok(s) = args.single::<String>() {
        parse_target(&s).ok_or(())
    } else {
        Err(())
    }
}

/// Robustly check if a message author is immune (cache -> partial member -> HTTP fetch).
pub async fn check_user_immune(ctx: &Context, msg: &Message, cfg: &crate::config::Config) -> bool {
    if msg.author.bot {
        return true;
    }
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return false,
    };

    // 1. Check cache
    if let Some(guild) = ctx.cache.guild(guild_id) {
        if let Some(member) = guild.members.get(&msg.author.id) {
            return cfg.is_immune(member);
        }
    }

    // 2. Check PartialMember attached to message
    if cfg.is_immune_partial(&msg.author, msg.member.as_deref()) {
        return true;
    }

    // 3. Fallback to HTTP fetch
    if let Ok(member) = guild_id.member(&ctx.http, msg.author.id).await {
        return cfg.is_immune(&member);
    }

    false
}

/// Robustly check if a message author is a moderator (cache -> partial member -> HTTP fetch).
pub async fn check_user_mod(ctx: &Context, msg: &Message, cfg: &crate::config::Config) -> bool {
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return false,
    };

    // 1. Check cache
    if let Some(guild) = ctx.cache.guild(guild_id) {
        if let Some(member) = guild.members.get(&msg.author.id) {
            return cfg.is_mod(member);
        }
    }

    // 2. Check PartialMember attached to message
    if cfg.is_mod_partial(msg.member.as_deref()) {
        return true;
    }

    // 3. Fallback to HTTP fetch
    if let Ok(member) = guild_id.member(&ctx.http, msg.author.id).await {
        return cfg.is_mod(&member);
    }

    false
}
