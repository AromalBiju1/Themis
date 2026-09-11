use serenity::{
    all::*,
    framework::standard::{macros::{command, group}, Args, CommandResult},
};
use crate::utils::check_user_mod;

#[group]
#[commands(embed)]
pub struct EmbedCmds;

#[command]
#[description = "Send a custom embed: $embed #channel title=\"Rules\" desc=\"Welcome!\" color=\"#FFB6C1\" image=\"url\""]
pub async fn embed(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let cfg = &data.get::<crate::BotData>().unwrap().config;

    if !check_user_mod(ctx, msg, cfg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }

    let target_ch = match args.single::<ChannelId>() {
        Ok(ch) => ch,
        Err(_) => msg.channel_id,
    };

    let rest = args.rest().trim();
    if rest.is_empty() {
        msg.reply(&ctx.http, "ℹ️ Usage: `$embed #channel title=\"Rules\" desc=\"Welcome!\" color=\"#FFB6C1\"`").await?;
        return Ok(());
    }

    let mut title = String::new();
    let mut desc = String::new();
    let mut color_hex = String::new();
    let mut image_url = String::new();

    for part in rest.split_whitespace() {
        if let Some(val) = part.strip_prefix("title=\"").and_then(|s| s.strip_suffix('"')) {
            title = val.to_string();
        } else if let Some(val) = part.strip_prefix("desc=\"").and_then(|s| s.strip_suffix('"')) {
            desc = val.to_string();
        } else if let Some(val) = part.strip_prefix("color=\"").and_then(|s| s.strip_suffix('"')) {
            color_hex = val.to_string();
        } else if let Some(val) = part.strip_prefix("image=\"").and_then(|s| s.strip_suffix('"')) {
            image_url = val.to_string();
        }
    }

    if desc.is_empty() && title.is_empty() {
        desc = rest.to_string();
    }

    let color = if color_hex.starts_with('#') {
        u32::from_str_radix(&color_hex[1..], 16).map(Color::new).unwrap_or(Color::BLURPLE)
    } else {
        Color::BLURPLE
    };

    let mut embed = CreateEmbed::new().color(color);
    if !title.is_empty() { embed = embed.title(title); }
    if !desc.is_empty() { embed = embed.description(desc); }
    if !image_url.is_empty() { embed = embed.image(image_url); }

    target_ch.send_message(&ctx.http, CreateMessage::new().embed(embed)).await?;
    msg.reply(&ctx.http, format!("✅ Sent custom embed to <#{}>.", target_ch)).await?;
    Ok(())
}
