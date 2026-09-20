use serenity::{
    all::*,
    framework::standard::{macros::{command, group}, Args, CommandResult},
};
use crate::{BotData, db, utils::check_user_mod};

#[group]
#[commands(youtube)]
pub struct YoutubeCmds;

#[command]
#[description = "Configure YouTube upload notifications: !youtube add <channel_id> [#discord_channel] [@ping_role] | remove <channel_id> | list"]
pub async fn youtube(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
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
            let input = args.single::<String>().unwrap_or_default();
            if input.is_empty() {
                msg.reply(&ctx.http, "❌ Usage: `$youtube add <youtube_channel_handle_or_id> [#discord_channel] [@ping_role]`").await?;
                return Ok(());
            }

            let target_ch = if let Some(ch) = crate::utils::parse_channel_from_args(ctx, guild_id, &mut args).await {
                ch
            } else {
                msg.channel_id
            };

            let ping_role = args.single::<RoleId>().ok().map(|r| r.get() as i64);

            let (ch_id, ch_name, latest_video) = match crate::handlers::youtube::resolve_youtube_channel(&input).await {
                Ok(res) => res,
                Err(e) => {
                    msg.reply(&ctx.http, format!("❌ {e}")).await?;
                    return Ok(());
                }
            };

            db::add_youtube_sub(pool, guild_id.get() as i64, &ch_id, target_ch.get() as i64, ping_role).await?;

            let mut posted_str = String::new();
            if let Some(ref video) = latest_video {
                if let Ok(subs) = db::list_youtube_subs(pool, guild_id.get() as i64).await {
                    if let Some(sub) = subs.iter().find(|s| s.youtube_channel_id == ch_id) {
                        let _ = crate::handlers::youtube::send_youtube_notification(&ctx.http, sub, video).await;
                        let _ = db::update_youtube_sub_last_video(pool, sub.id, &video.video_id, None).await;
                        posted_str = "\n🎥 **Latest video posted automatically!**".to_string();
                    }
                }
            }

            let role_str = ping_role.map(|r| format!(" (pinging <@&{r}>)")).unwrap_or_default();
            msg.reply(&ctx.http, format!("✅ Subscribed to YouTube channel **{ch_name}** (`{ch_id}`) in <#{target_ch}>{role_str}.{posted_str}\nNew uploads will be posted automatically.")).await?;
        }
        "remove" | "del" | "delete" => {
            let input = args.single::<String>().unwrap_or_default();
            if input.is_empty() {
                msg.reply(&ctx.http, "❌ Usage: `$youtube remove <youtube_channel_id_or_handle>`").await?;
                return Ok(());
            }

            let ch_id = match crate::handlers::youtube::resolve_youtube_channel(&input).await {
                Ok((id, _, _)) => id,
                Err(_) => input.clone(),
            };

            let removed = db::remove_youtube_sub(pool, guild_id.get() as i64, &ch_id).await?;
            if removed {
                msg.reply(&ctx.http, format!("✅ Unsubscribed from YouTube channel `{ch_id}`.")).await?;
            } else {
                msg.reply(&ctx.http, format!("❌ Subscription for `{input}` not found.")).await?;
            }
        }
        "test" => {
            let subs = db::list_youtube_subs(pool, guild_id.get() as i64).await?;
            if subs.is_empty() {
                msg.reply(&ctx.http, "❌ No YouTube subscriptions configured yet. Use `$youtube add <channel_handle>` first.").await?;
                return Ok(());
            }

            let mut count = 0;
            for sub in &subs {
                if let Ok((resolved_id, _, Some(video))) = crate::handlers::youtube::resolve_youtube_channel(&sub.youtube_channel_id).await {
                    if !(sub.youtube_channel_id.starts_with("UC") && sub.youtube_channel_id.len() == 24) {
                        let _ = db::update_youtube_sub_channel_id(pool, sub.id, &resolved_id).await;
                    }
                    if let Ok(_) = crate::handlers::youtube::send_youtube_notification(&ctx.http, sub, &video).await {
                        count += 1;
                    }
                }
            }

            if count > 0 {
                msg.reply(&ctx.http, format!("✅ Sent test notification preview for {count} YouTube subscription(s).")).await?;
            } else {
                msg.reply(&ctx.http, "❌ Could not fetch latest video for test preview.").await?;
            }
        }
        "list" | "show" => {
            let subs = db::list_youtube_subs(pool, guild_id.get() as i64).await?;
            if subs.is_empty() {
                msg.reply(&ctx.http, "ℹ️ No active YouTube subscriptions for this server.").await?;
                return Ok(());
            }

            let mut fields = Vec::new();
            for s in subs {
                let role_str = s.ping_role_id.map(|r| format!(" (Ping: <@&{r}>)")).unwrap_or_default();
                fields.push((
                    format!("Channel ID: {}", s.youtube_channel_id),
                    format!("Posting to: <#{}>{role_str}", s.discord_channel_id),
                    false,
                ));
            }

            let embed = CreateEmbed::new()
                .title("🎥 YouTube Subscriptions")
                .color(Color::from_rgb(255, 0, 0))
                .fields(fields);

            msg.channel_id.send_message(&ctx.http, CreateMessage::new().embed(embed)).await?;
        }
        _ => {
            msg.reply(
                &ctx.http,
                "ℹ️ Usage:\n• `$youtube add <handle_or_id> [#discord_channel] [@ping_role]`\n• `$youtube remove <handle_or_id>`\n• `$youtube test` - Send test preview\n• `$youtube list`",
            ).await?;
        }
    }

    Ok(())
}

/// Handle Slash Command Interactions
pub async fn handle_interaction(ctx: &Context, interaction: Interaction) {
    let command = match interaction.as_command() {
        Some(cmd) => cmd,
        None => return,
    };

    if command.data.name.as_str() != "youtube" {
        return;
    }

    let data = ctx.data.read().await;
    let bot_data = data.get::<BotData>().unwrap();

    let guild_id = match command.guild_id {
        Some(g) => g,
        None => return,
    };

    let options = &command.data.options;
    if let Some(sub) = options.first() {
        match sub.name.as_str() {
            "add" => {
                let mut raw_yt = None;
                let mut target_ch = None;
                let mut ping_role = None;

                if let CommandDataOptionValue::SubCommand(ref sub_opts) = sub.value {
                    for o in sub_opts {
                        match &o.value {
                            CommandDataOptionValue::String(s) => raw_yt = Some(s.as_str()),
                            CommandDataOptionValue::Channel(c) => target_ch = Some(*c),
                            CommandDataOptionValue::Role(r) => ping_role = Some(r.get() as i64),
                            _ => {}
                        }
                    }
                }

                if let (Some(yt_input), Some(ch)) = (raw_yt, target_ch) {
                    match crate::handlers::youtube::resolve_youtube_channel(yt_input).await {
                        Ok((ch_id, ch_name, latest_video)) => {
                            let _ = db::add_youtube_sub(&bot_data.db, guild_id.get() as i64, &ch_id, ch.get() as i64, ping_role).await;
                            let mut posted_str = String::new();
                            if let Some(ref video) = latest_video {
                                if let Ok(subs) = db::list_youtube_subs(&bot_data.db, guild_id.get() as i64).await {
                                    if let Some(sub) = subs.iter().find(|s| s.youtube_channel_id == ch_id) {
                                        let _ = crate::handlers::youtube::send_youtube_notification(&ctx.http, sub, video).await;
                                        let _ = db::update_youtube_sub_last_video(&bot_data.db, sub.id, &video.video_id, None).await;
                                        posted_str = "\n🎥 **Latest video posted automatically!**".to_string();
                                    }
                                }
                            }
                            let role_str = ping_role.map(|r| format!(" (pinging <@&{r}>)")).unwrap_or_default();
                            let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new().content(format!("✅ Subscribed to YouTube channel **{ch_name}** (`{ch_id}`) in <#{ch}>{role_str}.{posted_str}\nNew uploads will be posted automatically.")).ephemeral(true)
                            )).await;
                        }
                        Err(e) => {
                            let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new().content(format!("❌ {e}")).ephemeral(true)
                            )).await;
                        }
                    }
                } else {
                    let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content("❌ Please provide a valid YouTube channel handle/link and Discord channel.").ephemeral(true)
                    )).await;
                }
            }
            "remove" => {
                let raw_yt = if let CommandDataOptionValue::SubCommand(ref sub_opts) = sub.value {
                    sub_opts.iter().find_map(|o| match &o.value {
                        CommandDataOptionValue::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                } else {
                    None
                };

                if let Some(yt_input) = raw_yt {
                    let ch_id = match crate::handlers::youtube::resolve_youtube_channel(yt_input).await {
                        Ok((id, _, _)) => id,
                        Err(_) => yt_input.to_string(),
                    };

                    let removed = db::remove_youtube_sub(&bot_data.db, guild_id.get() as i64, &ch_id).await.unwrap_or(false);
                    let resp = if removed {
                        format!("✅ Unsubscribed from YouTube channel `{ch_id}`.")
                    } else {
                        format!("❌ Subscription for `{yt_input}` not found.")
                    };
                    let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(resp).ephemeral(true)
                    )).await;
                }
            }
            "test" => {
                let subs = db::list_youtube_subs(&bot_data.db, guild_id.get() as i64).await.unwrap_or_default();
                if subs.is_empty() {
                    let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content("❌ No active YouTube subscriptions found. Add one with `/youtube add` first.").ephemeral(true)
                    )).await;
                    return;
                }

                let mut count = 0;
                for sub in &subs {
                    if let Ok((resolved_id, _, Some(video))) = crate::handlers::youtube::resolve_youtube_channel(&sub.youtube_channel_id).await {
                        if !(sub.youtube_channel_id.starts_with("UC") && sub.youtube_channel_id.len() == 24) {
                            let _ = db::update_youtube_sub_channel_id(&bot_data.db, sub.id, &resolved_id).await;
                        }
                        if let Ok(_) = crate::handlers::youtube::send_youtube_notification(&ctx.http, sub, &video).await {
                            count += 1;
                        }
                    }
                }

                if count > 0 {
                    let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(format!("✅ Sent test notification preview for {count} YouTube subscription(s).")).ephemeral(true)
                    )).await;
                } else {
                    let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content("❌ Could not fetch latest video for preview.").ephemeral(true)
                    )).await;
                }
            }
            "list" => {
                let subs = db::list_youtube_subs(&bot_data.db, guild_id.get() as i64).await.unwrap_or_default();
                if subs.is_empty() {
                    let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content("ℹ️ No active YouTube subscriptions for this server.").ephemeral(true)
                    )).await;
                    return;
                }

                let mut fields = Vec::new();
                for s in subs {
                    let role_str = s.ping_role_id.map(|r| format!(" (Ping: <@&{r}>)")).unwrap_or_default();
                    fields.push((
                        format!("Channel ID: {}", s.youtube_channel_id),
                        format!("Posting to: <#{}>{role_str}", s.discord_channel_id),
                        false,
                    ));
                }

                let embed = CreateEmbed::new()
                    .title("🎥 YouTube Subscriptions")
                    .color(Color::from_rgb(255, 0, 0))
                    .fields(fields);

                let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(embed).ephemeral(true)
                )).await;
            }
            _ => {}
        }
    }
}
