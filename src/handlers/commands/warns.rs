use serenity::{
    all::*,
    framework::standard::{macros::{command, group}, Args, CommandResult},
};
use crate::{BotData, db, utils::{log_action, mod_embed}};
use chrono::Utc;

#[group]
#[commands(warn, warns, unwarn, clearwarns)]
pub struct WarnCmds;

// ── !warn ─────────────────────────────────────────────────────────────────────

#[command]
#[description = "Warn a member. Auto-escalates at configured thresholds."]
pub async fn warn(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    // Parse target before any async work
    let target_id = match args.single::<UserId>() {
        Ok(id) => id,
        Err(_) => { msg.reply(&ctx.http, "❌ Provide a member mention or ID.").await?; return Ok(()); }
    };
    let reason = args.rest().trim().to_string();
    let reason = if reason.is_empty() { "No reason provided".to_string() } else { reason };

    // Extract all cache data and drop CacheRef before any .await
    let (is_mod, target_is_admin, target_tag, mod_log_channel, guild_id, author_tag) = {
        let data = ctx.data.read().await;
        let cfg = &data.get::<BotData>().unwrap().config;
        let guild_ref = ctx.cache.guild(msg.guild_id.unwrap_or_default());
        let guild = match guild_ref.as_ref() {
            Some(g) => g,
            None => return Ok(()),
        };

        let is_mod = guild.members.get(&msg.author.id)
            .map(|m| cfg.is_mod(m))
            .unwrap_or(false);

        let (target_is_admin, target_tag) = guild.members.get(&target_id)
            .map(|m| (
                m.permissions.map(|p| p.administrator()).unwrap_or(false),
                m.user.tag(),
            ))
            .unwrap_or((false, target_id.to_string()));

        (is_mod, target_is_admin, target_tag, cfg.mod_log_channel, msg.guild_id.unwrap(), msg.author.tag())
    }; // CacheRef dropped here

    if !is_mod {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }
    if target_is_admin {
        msg.reply(&ctx.http, "❌ Cannot warn an admin.").await?;
        return Ok(());
    }

    let ts = Utc::now().to_rfc3339();

    let (pool, warn_mute_at, warn_kick_at, warn_ban_at) = {
        let data = ctx.data.read().await;
        let bd = data.get::<BotData>().unwrap();
        (bd.db.clone(), bd.config.warn_mute_at, bd.config.warn_kick_at, bd.config.warn_ban_at)
    };

    db::add_warn(
        &pool,
        guild_id.get() as i64,
        target_id.get() as i64,
        msg.author.id.get() as i64,
        &reason,
        &ts,
    ).await?;

    let count = db::count_warns(&pool, guild_id.get() as i64, target_id.get() as i64).await?;

    msg.reply(&ctx.http, format!("⚠️ <@{target_id}> warned. Total warns: **{count}**")).await?;

    let embed = mod_embed("⚠️ Member Warned", Color::GOLD, &[
        ("User",        &format!("{target_tag} (`{target_id}`)") as &str),
        ("Moderator",   &author_tag),
        ("Reason",      &reason),
        ("Total Warns", &count.to_string()),
    ]);
    log_action(&ctx.http, mod_log_channel, embed).await;

    // Escalation
    escalate(ctx, guild_id, target_id, count, warn_mute_at, warn_kick_at, warn_ban_at, msg).await;

    Ok(())
}

async fn escalate(
    ctx:          &Context,
    guild_id:     GuildId,
    user_id:      UserId,
    count:        u32,
    warn_mute_at: u32,
    warn_kick_at: u32,
    warn_ban_at:  u32,
    msg:          &Message,
) {
    if count >= warn_ban_at {
        let _ = guild_id.ban_with_reason(&ctx.http, user_id, 0, &format!("Reached {warn_ban_at} warns")).await;
        let _ = msg.reply(&ctx.http, format!("🔨 <@{user_id}> banned after {warn_ban_at} warns.")).await;
    } else if count >= warn_kick_at {
        let _ = guild_id.kick_with_reason(&ctx.http, user_id, &format!("Reached {warn_kick_at} warns")).await;
        let _ = msg.reply(&ctx.http, format!("👢 <@{user_id}> kicked after {warn_kick_at} warns.")).await;
    } else if count >= warn_mute_at {
        let until = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let _ = guild_id
            .edit_member(&ctx.http, user_id, EditMember::new().disable_communication_until(until))
            .await;
        let _ = msg.reply(&ctx.http, format!("🔇 <@{user_id}> timed out (1hr) after {warn_mute_at} warns.")).await;
    }
}

// ── !warns ────────────────────────────────────────────────────────────────────

#[command]
#[description = "List warns for a member."]
pub async fn warns(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let target_id = match args.single::<UserId>() {
        Ok(id) => id,
        Err(_) => { msg.reply(&ctx.http, "❌ Provide a member mention or ID.").await?; return Ok(()); }
    };

    let (is_mod, _mod_log_channel, guild_id) = {
        let data = ctx.data.read().await;
        let cfg = &data.get::<BotData>().unwrap().config;
        let is_mod = ctx.cache
            .guild(msg.guild_id.unwrap_or_default())
            .and_then(|g| g.members.get(&msg.author.id).map(|m| cfg.is_mod(m)))
            .unwrap_or(false);
        (is_mod, cfg.mod_log_channel, msg.guild_id.unwrap())
    };

    if !is_mod {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }

    let pool = {
        let data = ctx.data.read().await;
        data.get::<BotData>().unwrap().db.clone()
    };

    let rows = db::get_warns(&pool, guild_id.get() as i64, target_id.get() as i64).await?;

    if rows.is_empty() {
        msg.reply(&ctx.http, format!("✅ <@{target_id}> has no warns.")).await?;
        return Ok(());
    }

    let mut embed = CreateEmbed::new()
        .title(format!("Warns for <@{target_id}>"))
        .color(Color::GOLD);

    for w in &rows {
        embed = embed.field(
            format!("#{} — {}", w.id, &w.ts[..10.min(w.ts.len())]),
            format!("Mod: <@{}>\nReason: {}", w.mod_id, w.reason),
            false,
        );
    }
    msg.channel_id.send_message(&ctx.http, CreateMessage::new().embed(embed)).await?;
    Ok(())
}

// ── !unwarn ───────────────────────────────────────────────────────────────────

#[command]
#[description = "Remove a specific warn by ID."]
pub async fn unwarn(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let warn_id: i64 = match args.single() {
        Ok(id) => id,
        Err(_) => { msg.reply(&ctx.http, "❌ Provide a warn ID.").await?; return Ok(()); }
    };

    let is_mod = {
        let data = ctx.data.read().await;
        let cfg = &data.get::<BotData>().unwrap().config;
        ctx.cache
            .guild(msg.guild_id.unwrap_or_default())
            .and_then(|g| g.members.get(&msg.author.id).map(|m| cfg.is_mod(m)))
            .unwrap_or(false)
    };

    if !is_mod {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }

    let pool = {
        let data = ctx.data.read().await;
        data.get::<BotData>().unwrap().db.clone()
    };

    db::remove_warn(&pool, warn_id).await?;
    msg.reply(&ctx.http, format!("✅ Warn #{warn_id} removed.")).await?;
    Ok(())
}

// ── !clearwarns ───────────────────────────────────────────────────────────────

#[command]
#[description = "Clear all warns for a member."]
pub async fn clearwarns(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let target_id = match args.single::<UserId>() {
        Ok(id) => id,
        Err(_) => { msg.reply(&ctx.http, "❌ Provide a member mention or ID.").await?; return Ok(()); }
    };

    let (is_mod, guild_id) = {
        let data = ctx.data.read().await;
        let cfg = &data.get::<BotData>().unwrap().config;
        let is_mod = ctx.cache
            .guild(msg.guild_id.unwrap_or_default())
            .and_then(|g| g.members.get(&msg.author.id).map(|m| cfg.is_mod(m)))
            .unwrap_or(false);
        (is_mod, msg.guild_id.unwrap())
    };

    if !is_mod {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }

    let pool = {
        let data = ctx.data.read().await;
        data.get::<BotData>().unwrap().db.clone()
    };

    db::clear_warns(&pool, guild_id.get() as i64, target_id.get() as i64).await?;
    msg.reply(&ctx.http, format!("✅ All warns cleared for <@{target_id}>.")).await?;
    Ok(())
}
