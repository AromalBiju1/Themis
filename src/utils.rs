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
