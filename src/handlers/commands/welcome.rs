use serenity::{
    all::*,
    framework::standard::{macros::{command, group}, Args, CommandResult},
};
use crate::{BotData, db, utils::check_user_mod, handlers::welcome::send_welcome_embed};

#[group]
#[commands(welcome, welcometest)]
pub struct WelcomeCmds;

#[command]
#[description = "Configure welcome settings: !welcome channel #ch | text <msg> | image <url> | toggle | status"]
pub async fn welcome(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
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
        "channel" | "ch" => {
            let target_ch = msg.channel_id;
            let ch_id = if let Ok(parsed) = args.single::<ChannelId>() {
                parsed
            } else {
                target_ch
            };
            db::set_welcome_channel(pool, guild_id.get() as i64, ch_id.get() as i64).await?;
            msg.reply(&ctx.http, format!("✅ Welcome channel set to <#{}>.", ch_id)).await?;
        }
        "text" | "msg" => {
            let text = args.rest().trim();
            if text.is_empty() {
                msg.reply(&ctx.http, "❌ Please provide welcome text.").await?;
                return Ok(());
            }
            db::set_welcome_text(pool, guild_id.get() as i64, text).await?;
            msg.reply(&ctx.http, "✅ Welcome message text updated.").await?;
        }
        "image" | "img" => {
            let url = args.single::<String>().unwrap_or_default();
            db::set_welcome_image(pool, guild_id.get() as i64, &url).await?;
            msg.reply(&ctx.http, "✅ Welcome banner image updated.").await?;
        }
        "toggle" => {
            let enabled = db::toggle_welcome(pool, guild_id.get() as i64).await?;
            let status = if enabled { "enabled" } else { "disabled" };
            msg.reply(&ctx.http, format!("✅ Welcome messages are now **{}**.", status)).await?;
        }
        "status" | "show" => {
            let config = db::get_welcome_config(pool, guild_id.get() as i64).await?;
            if let Some(c) = config {
                let status_str = if c.enabled { "Enabled" } else { "Disabled" };
                let ch_str = if c.channel_id > 0 { format!("<#{}>", c.channel_id) } else { "Not set".to_string() };
                let img_str = c.image_url.unwrap_or_else(|| "None".to_string());

                let embed = CreateEmbed::new()
                    .title("⚙️ Welcome Settings")
                    .color(Color::BLURPLE)
                    .field("Status", status_str, true)
                    .field("Channel", ch_str, true)
                    .field("Image Banner", img_str, false)
                    .field("Text Template", format!("```\n{}\n```", c.message), false);

                msg.channel_id.send_message(&ctx.http, CreateMessage::new().embed(embed)).await?;
            } else {
                msg.reply(&ctx.http, "⚠️ No welcome configuration set yet. Use `!welcome channel #ch` to start.").await?;
            }
        }
        _ => {
            msg.reply(
                &ctx.http,
                "ℹ️ Usage:\n• `!welcome channel #channel` - Set welcome channel\n• `!welcome text <message>` - Set welcome text\n• `!welcome image <url>` - Set banner image URL\n• `!welcome toggle` - Enable/Disable\n• `!welcome status` - View settings\n• `!welcometest` - Send preview",
            ).await?;
        }
    }

    Ok(())
}

#[command]
#[description = "Send a test welcome message embed in the configured channel."]
pub async fn welcometest(ctx: &Context, msg: &Message) -> CommandResult {
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

    let welcome_cfg = db::get_welcome_config(&bot_data.db, guild_id.get() as i64).await?;
    let welcome_cfg = match welcome_cfg {
        Some(c) => c,
        None => {
            msg.reply(&ctx.http, "❌ Welcome channel is not set yet. Use `!welcome channel #channel` first.").await?;
            return Ok(());
        }
    };

    let target_ch = if welcome_cfg.channel_id > 0 {
        ChannelId::new(welcome_cfg.channel_id as u64)
    } else {
        msg.channel_id
    };

    send_welcome_embed(ctx, guild_id, &msg.author, target_ch, &welcome_cfg).await;
    msg.reply(&ctx.http, format!("✅ Sent test welcome message to <#{}>.", target_ch)).await?;
    Ok(())
}

/// Register slash commands in Discord API
pub async fn register_slash_commands(ctx: &Context) {
    let welcome_cmd = CreateCommand::new("welcome")
        .description("Configure welcome messages")
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "set_channel", "Set the welcome channel")
                .add_sub_option(CreateCommandOption::new(CommandOptionType::Channel, "channel", "Select welcome channel").required(true))
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "set_text", "Set the welcome message text")
                .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "message", "Welcome text template").required(true))
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "set_image", "Set the bottom banner image URL")
                .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "url", "Direct image URL").required(true))
        )
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "toggle", "Toggle welcome messages on/off"))
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "status", "View current welcome settings"));

    let test_cmd = CreateCommand::new("welcometest")
        .description("Send a test welcome message in the designated channel");

    if let Err(e) = Command::set_global_commands(&ctx.http, vec![welcome_cmd, test_cmd]).await {
        tracing::error!("Failed to register slash commands: {e}");
    } else {
        tracing::info!("Registered slash commands: /welcome, /welcometest");
    }
}

/// Handle Slash Command Interactions
pub async fn handle_interaction(ctx: &Context, interaction: Interaction) {
    let command = match interaction.as_command() {
        Some(cmd) => cmd,
        None => return,
    };

    let data = ctx.data.read().await;
    let bot_data = data.get::<BotData>().unwrap();

    let guild_id = match command.guild_id {
        Some(g) => g,
        None => return,
    };

    match command.data.name.as_str() {
        "welcometest" => {
            let welcome_cfg = match db::get_welcome_config(&bot_data.db, guild_id.get() as i64).await {
                Ok(Some(c)) => c,
                _ => {
                    let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content("❌ Welcome settings not configured yet. Use `/welcome set_channel #channel` first.").ephemeral(true)
                    )).await;
                    return;
                }
            };
            let target_ch = if welcome_cfg.channel_id > 0 { ChannelId::new(welcome_cfg.channel_id as u64) } else { command.channel_id };
            send_welcome_embed(ctx, guild_id, &command.user, target_ch, &welcome_cfg).await;
            let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(format!("✅ Test welcome message sent to <#{}>!", target_ch)).ephemeral(true)
            )).await;
        }
        "welcome" => {
            let options = &command.data.options;
            if let Some(sub) = options.first() {
                match sub.name.as_str() {
                    "set_channel" => {
                        let ch_id = sub.value.as_channel_id().or_else(|| {
                            // In Serenity 0.12, suboptions may be inside sub.options or ResolvedOption
                            None
                        });
                        let ch = if let CommandDataOptionValue::SubCommand(ref sub_opts) = sub.value {
                            sub_opts.iter().find_map(|o| match &o.value {
                                CommandDataOptionValue::Channel(c) => Some(*c),
                                _ => None,
                            })
                        } else {
                            ch_id
                        };

                        if let Some(c) = ch {
                            let _ = db::set_welcome_channel(&bot_data.db, guild_id.get() as i64, c.get() as i64).await;
                            let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new().content(format!("✅ Welcome channel set to <#{}>.", c)).ephemeral(true)
                            )).await;
                        } else {
                            let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new().content("❌ Please select a valid channel.").ephemeral(true)
                            )).await;
                        }
                    }
                    "set_text" => {
                        let msg_text = if let CommandDataOptionValue::SubCommand(ref sub_opts) = sub.value {
                            sub_opts.iter().find_map(|o| match &o.value {
                                CommandDataOptionValue::String(s) => Some(s.as_str()),
                                _ => None,
                            })
                        } else {
                            None
                        };

                        if let Some(text) = msg_text {
                            let _ = db::set_welcome_text(&bot_data.db, guild_id.get() as i64, text).await;
                            let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new().content("✅ Welcome message text updated.").ephemeral(true)
                            )).await;
                        } else {
                            let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new().content("❌ Please provide welcome message text.").ephemeral(true)
                            )).await;
                        }
                    }
                    "set_image" => {
                        let img_url = if let CommandDataOptionValue::SubCommand(ref sub_opts) = sub.value {
                            sub_opts.iter().find_map(|o| match &o.value {
                                CommandDataOptionValue::String(s) => Some(s.as_str()),
                                _ => None,
                            })
                        } else {
                            None
                        };

                        if let Some(url) = img_url {
                            let _ = db::set_welcome_image(&bot_data.db, guild_id.get() as i64, url).await;
                            let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new().content("✅ Welcome banner image updated.").ephemeral(true)
                            )).await;
                        } else {
                            let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new().content("❌ Please provide an image URL.").ephemeral(true)
                            )).await;
                        }
                    }
                    "toggle" => {
                        let enabled = db::toggle_welcome(&bot_data.db, guild_id.get() as i64).await.unwrap_or(false);
                        let status = if enabled { "enabled" } else { "disabled" };
                        let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().content(format!("✅ Welcome messages are now **{}**.", status)).ephemeral(true)
                        )).await;
                    }
                    "status" => {
                        if let Ok(Some(c)) = db::get_welcome_config(&bot_data.db, guild_id.get() as i64).await {
                            let embed = CreateEmbed::new()
                                .title("⚙️ Welcome Settings")
                                .color(Color::BLURPLE)
                                .field("Status", if c.enabled { "Enabled" } else { "Disabled" }, true)
                                .field("Channel", if c.channel_id > 0 { format!("<#{}>", c.channel_id) } else { "Not set".to_string() }, true)
                                .field("Image Banner", c.image_url.unwrap_or_else(|| "None".to_string()), false)
                                .field("Text Template", format!("```\n{}\n```", c.message), false);

                            let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new().embed(embed).ephemeral(true)
                            )).await;
                        } else {
                            let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new().content("⚠️ No welcome settings configured.").ephemeral(true)
                            )).await;
                        }
                    }
                    _ => {
                        let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().content("✅ Setting updated.").ephemeral(true)
                        )).await;
                    }
                }
            }
        }
        _ => {}
    }
}
