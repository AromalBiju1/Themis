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
