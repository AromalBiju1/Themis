use serenity::{
    all::*,
    framework::standard::{macros::{command, group}, Args, CommandResult},
};
use crate::{BotData, db, utils::check_user_mod, handlers::goodbye::send_goodbye_embed};

#[group]
#[commands(goodbye, goodbyetest)]
pub struct GoodbyeCmds;

#[command]
#[description = "Configure goodbye settings: $goodbye channel #ch | text <msg> | toggle | status"]
pub async fn goodbye(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
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

    let mut sub = args.single::<String>().unwrap_or_default().to_lowercase();
    if sub == "set" {
        sub = args.single::<String>().unwrap_or_default().to_lowercase();
    }
    let pool = &bot_data.db;

    match sub.as_str() {
        "channel" | "ch" | "set_channel" | "setchannel" => {
            if let Some(target_ch) = crate::utils::parse_channel_from_args(ctx, guild_id, &mut args) {
                db::set_goodbye_channel(pool, guild_id.get() as i64, target_ch.get() as i64).await?;
                msg.reply(&ctx.http, format!("✅ Goodbye channel set to <#{}>.", target_ch)).await?;
            } else {
                msg.reply(&ctx.http, "❌ Please specify a valid channel (e.g. `$goodbye channel #goodbye-channel` or `$goodbye channel 123456789`).").await?;
            }
        }
        "text" | "msg" | "set_text" | "settext" => {
            let text = args.rest().trim();
            if text.is_empty() {
                msg.reply(&ctx.http, "❌ Please provide goodbye text.").await?;
                return Ok(());
            }
            db::set_goodbye_text(pool, guild_id.get() as i64, text).await?;
            msg.reply(&ctx.http, "✅ Goodbye message text updated.").await?;
        }
        "toggle" => {
            let enabled = db::toggle_goodbye(pool, guild_id.get() as i64).await?;
            let status = if enabled { "enabled" } else { "disabled" };
            msg.reply(&ctx.http, format!("✅ Goodbye messages are now **{}**.", status)).await?;
        }
        _ => {
            msg.reply(&ctx.http, "ℹ️ Usage: `$goodbye channel #ch` | `$goodbye text <msg>` | `$goodbye toggle` | `$goodbyetest`").await?;
        }
    }

    Ok(())
}

#[command]
#[description = "Send a test goodbye message embed."]
pub async fn goodbyetest(ctx: &Context, msg: &Message) -> CommandResult {
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

    let goodbye_cfg = db::get_goodbye_config(&bot_data.db, guild_id.get() as i64).await?;
    let goodbye_cfg = match goodbye_cfg {
        Some(c) => c,
        None => {
            msg.reply(&ctx.http, "❌ Goodbye channel is not set yet. Use `$goodbye channel #channel` first.").await?;
            return Ok(());
        }
    };

    let target_ch = if goodbye_cfg.channel_id > 0 { ChannelId::new(goodbye_cfg.channel_id as u64) } else { msg.channel_id };
    send_goodbye_embed(ctx, guild_id, &msg.author, target_ch, &goodbye_cfg).await;
    msg.reply(&ctx.http, format!("✅ Sent test goodbye message to <#{}>.", target_ch)).await?;
    Ok(())
}
