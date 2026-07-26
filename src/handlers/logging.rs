use serenity::all::*;
use chrono::Utc;
use crate::{BotData, utils::log_action};

fn utcnow() -> Timestamp {
    Timestamp::from(Utc::now())
}

/// on_guild_member_addition
pub async fn handle_member_join(ctx: &Context, member: &Member) {
    let mod_log_channel = {
        let data = ctx.data.read().await;
        data.get::<BotData>().unwrap().config.mod_log_channel
    };

    let account_created = format!("<t:{}:R>", member.user.created_at().unix_timestamp());

    // Get member count from cache (no await needed)
    let member_count = ctx.cache
        .guild(member.guild_id)
        .map(|g| g.member_count)
        .unwrap_or(0);

    let avatar_url = member.user.face();
    let user_tag   = member.user.tag();
    let user_id    = member.user.id;

    let embed = CreateEmbed::new()
        .title("📥 Member Joined")
        .color(Color::DARK_GREEN)
        .timestamp(utcnow())
        .thumbnail(avatar_url)
        .field("User",            format!("{user_tag} (`{user_id}`)"), false)
        .field("Account Created", account_created, false)
        .footer(CreateEmbedFooter::new(format!("Member count: {member_count}")));

    log_action(&ctx.http, mod_log_channel, embed).await;
}

/// on_guild_member_removal
pub async fn handle_member_leave(ctx: &Context, _guild_id: GuildId, user: &User, member_data: &Option<Member>) {
    let mod_log_channel = {
        let data = ctx.data.read().await;
        data.get::<BotData>().unwrap().config.mod_log_channel
    };

    let roles_str = member_data
        .as_ref()
        .map(|m| {
            let r: Vec<String> = m.roles.iter().map(|rid| format!("<@&{}>", rid)).collect();
            if r.is_empty() { "None".to_string() } else { r.join(", ") }
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let embed = CreateEmbed::new()
        .title("📤 Member Left")
        .color(Color::LIGHT_GREY)
        .timestamp(utcnow())
        .field("User",  format!("{} (`{}`)", user.tag(), user.id), false)
        .field("Roles", roles_str, false);

    log_action(&ctx.http, mod_log_channel, embed).await;
}

/// on_message_update
pub async fn handle_message_edit(
    ctx:    &Context,
    old:    &Option<Message>,
    new:    &Option<Message>,
    _event: &MessageUpdateEvent,
) {
    let (before, after) = match (old, new) {
        (Some(b), Some(a)) => (b, a),
        _ => return,
    };
    if before.author.bot || before.guild_id.is_none() {
        return;
    }
    if before.content == after.content {
        return;
    }

    let mod_log_channel = {
        let data = ctx.data.read().await;
        data.get::<BotData>().unwrap().config.mod_log_channel
    };

    let embed = CreateEmbed::new()
        .title("✏️ Message Edited")
        .color(Color::BLUE)
        .timestamp(utcnow())
        .field("Author",  format!("{} (`{}`)", before.author.tag(), before.author.id), false)
        .field("Channel", format!("<#{}>", before.channel_id), false)
        .field("Before",  truncate(&before.content, 1024), false)
        .field("After",   truncate(&after.content, 1024), false)
        .field("Jump",    format!("[Link]({})", after.link()), false);

    log_action(&ctx.http, mod_log_channel, embed).await;
}

/// on_message_delete
pub async fn handle_message_delete(
    ctx:        &Context,
    channel_id: ChannelId,
    deleted_id: MessageId,
    guild_id:   Option<GuildId>,
) {
    if guild_id.is_none() {
        return;
    }

    let mod_log_channel = {
        let data = ctx.data.read().await;
        data.get::<BotData>().unwrap().config.mod_log_channel
    };

    // Best effort — message may not be in cache
    let (author_str, content_str) = ctx.cache
        .message(channel_id, deleted_id)
        .filter(|msg| !msg.author.bot)
        .map(|msg| (
            format!("{} (`{}`)", msg.author.tag(), msg.author.id),
            truncate(&msg.content, 1024),
        ))
        .unwrap_or_else(|| (
            format!("Unknown (msg `{deleted_id}`)"),
            "*not in cache*".to_string(),
        ));

    let embed = CreateEmbed::new()
        .title("🗑️ Message Deleted")
        .color(Color::RED)
        .timestamp(utcnow())
        .field("Author",  author_str, false)
        .field("Channel", format!("<#{channel_id}>"), false)
        .field("Content", content_str, false);

    log_action(&ctx.http, mod_log_channel, embed).await;
}

/// on_guild_ban_addition
pub async fn handle_ban(ctx: &Context, _guild_id: GuildId, user: &User) {
    let mod_log_channel = {
        let data = ctx.data.read().await;
        data.get::<BotData>().unwrap().config.mod_log_channel
    };

    let embed = CreateEmbed::new()
        .title("🔨 Member Banned")
        .color(Color::DARK_RED)
        .timestamp(utcnow())
        .field("User", format!("{} (`{}`)", user.tag(), user.id), false);

    log_action(&ctx.http, mod_log_channel, embed).await;
}

/// on_guild_ban_removal
pub async fn handle_unban(ctx: &Context, _guild_id: GuildId, user: &User) {
    let mod_log_channel = {
        let data = ctx.data.read().await;
        data.get::<BotData>().unwrap().config.mod_log_channel
    };

    let embed = CreateEmbed::new()
        .title("✅ Member Unbanned")
        .color(Color::DARK_GREEN)
        .timestamp(utcnow())
        .field("User", format!("{} (`{}`)", user.tag(), user.id), false);

    log_action(&ctx.http, mod_log_channel, embed).await;
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
