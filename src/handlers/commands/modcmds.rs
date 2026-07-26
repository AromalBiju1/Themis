use serenity::{
    all::*,
    framework::standard::{macros::{command, group}, Args, CommandResult},
};
use crate::{BotData, utils::{log_action, mod_embed}};

#[group]
#[commands(ban, kick, softban, mute, unmute, purge, slowmode, unban)]
pub struct ModCmds;

// ── Guard ─────────────────────────────────────────────────────────────────────

async fn check_mod(ctx: &Context, msg: &Message) -> bool {
    let data = ctx.data.read().await;
    let cfg = &data.get::<BotData>().unwrap().config;
    ctx.cache
        .guild(msg.guild_id.unwrap_or_default())
        .and_then(|g| g.members.get(&msg.author.id).map(|m| cfg.is_mod(m)))
        .unwrap_or(false)
}

// ── !ban ──────────────────────────────────────────────────────────────────────

#[command]
#[description = "Ban a member."]
pub async fn ban(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if !check_mod(ctx, msg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }
    let member = match args.single::<UserId>() {
        Ok(id) => {
            let guild_id = match msg.guild_id { Some(g) => g, None => return Ok(()) };
            match guild_id.member(&ctx.http, id).await {
                Ok(m) => m,
                Err(_) => { msg.reply(&ctx.http, "❌ Member not found.").await?; return Ok(()); }
            }
        }
        Err(_) => { msg.reply(&ctx.http, "❌ Provide a member mention or ID.").await?; return Ok(()); }
    };
    let reason = args.rest().trim().to_string();
    let reason = if reason.is_empty() { "No reason provided".to_string() } else { reason };

    match msg.guild_id.unwrap().ban_with_reason(&ctx.http, member.user.id, 0, &reason).await {
        Ok(_) => {
            msg.reply(&ctx.http, format!("🔨 Banned {}.", member.user.tag())).await?;
            let data = ctx.data.read().await;
            let cfg = &data.get::<BotData>().unwrap().config;
            let embed = mod_embed("🔨 Ban", Color::RED, &[
                ("User",      &format!("{} (`{}`)", member.user.tag(), member.user.id)),
                ("Moderator", &msg.author.tag()),
                ("Reason",    &reason),
            ]);
            log_action(&ctx.http, cfg.mod_log_channel, embed).await;
        }
        Err(_) => { msg.reply(&ctx.http, "❌ Missing permissions.").await?; }
    }
    Ok(())
}

// ── !kick ─────────────────────────────────────────────────────────────────────

#[command]
#[description = "Kick a member."]
pub async fn kick(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if !check_mod(ctx, msg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }
    let member = match args.single::<UserId>() {
        Ok(id) => {
            let guild_id = match msg.guild_id { Some(g) => g, None => return Ok(()) };
            match guild_id.member(&ctx.http, id).await {
                Ok(m) => m,
                Err(_) => { msg.reply(&ctx.http, "❌ Member not found.").await?; return Ok(()); }
            }
        }
        Err(_) => { msg.reply(&ctx.http, "❌ Provide a member mention or ID.").await?; return Ok(()); }
    };
    let reason = args.rest().trim().to_string();
    let reason = if reason.is_empty() { "No reason provided".to_string() } else { reason };

    match member.kick_with_reason(&ctx.http, &reason).await {
        Ok(_) => {
            msg.reply(&ctx.http, format!("👢 Kicked {}.", member.user.tag())).await?;
            let data = ctx.data.read().await;
            let cfg = &data.get::<BotData>().unwrap().config;
            let embed = mod_embed("👢 Kick", Color::ORANGE, &[
                ("User",      &format!("{} (`{}`)", member.user.tag(), member.user.id)),
                ("Moderator", &msg.author.tag()),
                ("Reason",    &reason),
            ]);
            log_action(&ctx.http, cfg.mod_log_channel, embed).await;
        }
        Err(_) => { msg.reply(&ctx.http, "❌ Missing permissions.").await?; }
    }
    Ok(())
}

// ── !softban ──────────────────────────────────────────────────────────────────

#[command]
#[description = "Ban + unban to wipe 24hr message history."]
pub async fn softban(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if !check_mod(ctx, msg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }
    let member = match args.single::<UserId>() {
        Ok(id) => {
            let guild_id = match msg.guild_id { Some(g) => g, None => return Ok(()) };
            match guild_id.member(&ctx.http, id).await {
                Ok(m) => m,
                Err(_) => { msg.reply(&ctx.http, "❌ Member not found.").await?; return Ok(()); }
            }
        }
        Err(_) => { msg.reply(&ctx.http, "❌ Provide a member mention or ID.").await?; return Ok(()); }
    };
    let reason = args.rest().trim().to_string();
    let reason = if reason.is_empty() { "No reason provided".to_string() } else { reason };
    let guild_id = msg.guild_id.unwrap();

    match guild_id.ban_with_reason(&ctx.http, member.user.id, 1, &format!("Softban: {reason}")).await {
        Ok(_) => {
            let _ = guild_id.unban(&ctx.http, member.user.id).await;
            msg.reply(&ctx.http, format!("🧹 Softbanned {} (messages wiped).", member.user.tag())).await?;
            let data = ctx.data.read().await;
            let cfg = &data.get::<BotData>().unwrap().config;
            let embed = mod_embed("🧹 Softban", Color::ORANGE, &[
                ("User",      &format!("{} (`{}`)", member.user.tag(), member.user.id)),
                ("Moderator", &msg.author.tag()),
                ("Reason",    &reason),
            ]);
            log_action(&ctx.http, cfg.mod_log_channel, embed).await;
        }
        Err(_) => { msg.reply(&ctx.http, "❌ Missing permissions.").await?; }
    }
    Ok(())
}

// ── !mute ─────────────────────────────────────────────────────────────────────

#[command]
#[description = "Timeout a member (default 60 minutes)."]
pub async fn mute(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if !check_mod(ctx, msg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }
    let member = match args.single::<UserId>() {
        Ok(id) => {
            let guild_id = match msg.guild_id { Some(g) => g, None => return Ok(()) };
            match guild_id.member(&ctx.http, id).await {
                Ok(m) => m,
                Err(_) => { msg.reply(&ctx.http, "❌ Member not found.").await?; return Ok(()); }
            }
        }
        Err(_) => { msg.reply(&ctx.http, "❌ Provide a member mention or ID.").await?; return Ok(()); }
    };
    let minutes: i64 = args.single().unwrap_or(60);
    let reason = args.rest().trim().to_string();
    let reason = if reason.is_empty() { "No reason provided".to_string() } else { reason };

    let until = (chrono::Utc::now() + chrono::Duration::minutes(minutes)).to_rfc3339();
    match member.guild_id
        .edit_member(&ctx.http, member.user.id, EditMember::new().disable_communication_until(until))
        .await
    {
        Ok(_) => {
            msg.reply(&ctx.http, format!("🔇 Muted {} for {minutes} minutes.", member.user.tag())).await?;
            let data = ctx.data.read().await;
            let cfg = &data.get::<BotData>().unwrap().config;
            let embed = mod_embed("🔇 Mute", Color::BLUE, &[
                ("User",      &format!("{} (`{}`)", member.user.tag(), member.user.id)),
                ("Moderator", &msg.author.tag()),
                ("Duration",  &format!("{minutes} minutes")),
                ("Reason",    &reason),
            ]);
            log_action(&ctx.http, cfg.mod_log_channel, embed).await;
        }
        Err(_) => { msg.reply(&ctx.http, "❌ Missing permissions.").await?; }
    }
    Ok(())
}

// ── !unmute ───────────────────────────────────────────────────────────────────

#[command]
#[description = "Remove a member's timeout."]
pub async fn unmute(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if !check_mod(ctx, msg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }
    let member = match args.single::<UserId>() {
        Ok(id) => {
            let guild_id = match msg.guild_id { Some(g) => g, None => return Ok(()) };
            match guild_id.member(&ctx.http, id).await {
                Ok(m) => m,
                Err(_) => { msg.reply(&ctx.http, "❌ Member not found.").await?; return Ok(()); }
            }
        }
        Err(_) => { msg.reply(&ctx.http, "❌ Provide a member mention or ID.").await?; return Ok(()); }
    };

    match member.guild_id
        .edit_member(&ctx.http, member.user.id, EditMember::new().enable_communication())
        .await
    {
        Ok(_) => { msg.reply(&ctx.http, format!("🔊 Unmuted {}.", member.user.tag())).await?; }
        Err(_) => { msg.reply(&ctx.http, "❌ Missing permissions.").await?; }
    }
    Ok(())
}

// ── !purge ────────────────────────────────────────────────────────────────────

#[command]
#[description = "Delete up to 500 messages. Optionally filter by user."]
pub async fn purge(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if !check_mod(ctx, msg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }
    let amount: u8 = match args.single::<u16>() {
        Ok(n) if n >= 1 && n <= 500 => n as u8,
        _ => { msg.reply(&ctx.http, "❌ Amount must be 1-500.").await?; return Ok(()); }
    };
    let filter_uid = args.single::<UserId>().ok();

    let messages = msg.channel_id
        .messages(&ctx.http, GetMessages::new().limit(amount))
        .await?;

    let to_delete: Vec<MessageId> = messages
        .iter()
        .filter(|m| filter_uid.map(|uid| m.author.id == uid).unwrap_or(true))
        .map(|m| m.id)
        .collect();

    let count = to_delete.len();
    msg.channel_id.delete_messages(&ctx.http, &to_delete).await?;
    msg.channel_id.say(&ctx.http, format!("🗑️ Deleted {count} messages.")).await?;
    Ok(())
}

// ── !slowmode ─────────────────────────────────────────────────────────────────

#[command]
#[description = "Set slowmode (seconds). 0 to disable."]
pub async fn slowmode(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if !check_mod(ctx, msg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }
    let secs: u16 = args.single().unwrap_or(0);
    // Look up channel from cache directly (not async)
    let ch_id = msg.channel_id;
    ch_id.edit(&ctx.http, EditChannel::new().rate_limit_per_user(secs)).await?;
    let text = if secs == 0 { "✅ Slowmode disabled.".to_string() }
               else          { format!("✅ Slowmode set to {secs}s.") };
    msg.reply(&ctx.http, text).await?;
    Ok(())
}

// ── !unban ────────────────────────────────────────────────────────────────────

#[command]
#[description = "Unban a user by ID."]
pub async fn unban(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if !check_mod(ctx, msg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }
    let user_id: UserId = match args.single() {
        Ok(id) => UserId::new(id),
        Err(_) => { msg.reply(&ctx.http, "❌ Provide a valid user ID.").await?; return Ok(()); }
    };
    let _reason = args.rest().trim().to_string();
    let guild_id = match msg.guild_id { Some(g) => g, None => return Ok(()) };

    match guild_id.unban(&ctx.http, user_id).await {
        Ok(_) => {
            let user = ctx.http.get_user(user_id).await?;
            msg.reply(&ctx.http, format!("✅ Unbanned {}.", user.tag())).await?;
        }
        Err(Error::Http(ref e)) if e.status_code().map(|c| c.as_u16()) == Some(404) => {
            msg.reply(&ctx.http, "❌ User not found or not banned.").await?;
        }
        Err(_) => { msg.reply(&ctx.http, "❌ Missing permissions.").await?; }
    }
    Ok(())
}
