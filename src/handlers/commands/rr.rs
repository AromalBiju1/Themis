use serenity::{
    all::*,
    framework::standard::{macros::{command, group}, Args, CommandResult},
};
use crate::{BotData, db, utils::check_user_mod};

#[group]
#[commands(rr)]
pub struct RRCmds;

#[command]
#[description = "Reaction Roles: $rr add #ch <msg_id> <emoji> @role | $rr remove #ch <msg_id> <emoji>"]
pub async fn rr(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let bot_data = data.get::<BotData>().unwrap();
    let cfg = &bot_data.config;

    if !check_user_mod(ctx, msg, cfg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }

    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return Ok(()),
    };

    let sub = args.single::<String>().unwrap_or_default().to_lowercase();
    let pool = &bot_data.db;

    match sub.as_str() {
        "add" => {
            let channel_id = match args.single::<ChannelId>() {
                Ok(ch) => ch,
                Err(_) => { msg.reply(&ctx.http, "❌ Provide a channel mention (e.g. `#roles`).").await?; return Ok(()); }
            };
            let msg_id = match args.single::<u64>() {
                Ok(id) => MessageId::new(id),
                Err(_) => { msg.reply(&ctx.http, "❌ Provide a valid message ID.").await?; return Ok(()); }
            };
            let emoji_str = match args.single::<String>() {
                Ok(e) => e,
                Err(_) => { msg.reply(&ctx.http, "❌ Provide an emoji.").await?; return Ok(()); }
            };
            let role_id = match args.single::<RoleId>() {
                Ok(r) => r,
                Err(_) => { msg.reply(&ctx.http, "❌ Provide a role mention or ID.").await?; return Ok(()); }
            };

            db::add_reaction_role(pool, guild_id.get() as i64, channel_id.get() as i64, msg_id.get() as i64, &emoji_str, role_id.get() as i64).await?;

            if let Ok(reaction_emoji) = ReactionType::try_from(emoji_str.as_str()) {
                let _ = ctx.http.create_reaction(channel_id, msg_id, &reaction_emoji).await;
            }

            msg.reply(&ctx.http, format!("✅ Added reaction role: {} -> <@&{}> on message `{}`.", emoji_str, role_id, msg_id)).await?;
        }
        "remove" | "delete" => {
            let _channel_id = match args.single::<ChannelId>() {
                Ok(ch) => ch,
                Err(_) => { msg.reply(&ctx.http, "❌ Provide a channel mention.").await?; return Ok(()); }
            };
            let msg_id = match args.single::<u64>() {
                Ok(id) => id,
                Err(_) => { msg.reply(&ctx.http, "❌ Provide a valid message ID.").await?; return Ok(()); }
            };
            let emoji_str = match args.single::<String>() {
                Ok(e) => e,
                Err(_) => { msg.reply(&ctx.http, "❌ Provide an emoji.").await?; return Ok(()); }
            };

            db::remove_reaction_role(pool, guild_id.get() as i64, msg_id as i64, &emoji_str).await?;
            msg.reply(&ctx.http, format!("✅ Removed reaction role for {} on message `{}`.", emoji_str, msg_id)).await?;
        }
        _ => {
            msg.reply(
                &ctx.http,
                "ℹ️ Usage:\n• `$rr add #channel <message_id> <emoji> @role` - Link emoji reaction to role\n• `$rr remove #channel <message_id> <emoji>` - Remove reaction role",
            ).await?;
        }
    }

    Ok(())
}
