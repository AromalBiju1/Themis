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
