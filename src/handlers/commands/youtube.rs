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
            let yt_id = args.single::<String>().unwrap_or_default();
            if yt_id.is_empty() {
                msg.reply(&ctx.http, "❌ Usage: `!youtube add <youtube_channel_id> [#discord_channel] [@ping_role]`").await?;
                return Ok(());
            }

            let target_ch = if let Ok(parsed) = args.single::<ChannelId>() {
                parsed
            } else {
                msg.channel_id
            };

            let ping_role = args.single::<RoleId>().ok().map(|r| r.get() as i64);

            db::add_youtube_sub(pool, guild_id.get() as i64, &yt_id, target_ch.get() as i64, ping_role).await?;
            let role_str = ping_role.map(|r| format!(" (pinging <@&{r}>)")).unwrap_or_default();
            msg.reply(&ctx.http, format!("✅ Subscribed to YouTube channel `{yt_id}` in <#{target_ch}>{role_str}.")).await?;
        }
        "remove" | "del" | "delete" => {
            let yt_id = args.single::<String>().unwrap_or_default();
            if yt_id.is_empty() {
                msg.reply(&ctx.http, "❌ Usage: `!youtube remove <youtube_channel_id>`").await?;
                return Ok(());
            }

            let removed = db::remove_youtube_sub(pool, guild_id.get() as i64, &yt_id).await?;
            if removed {
                msg.reply(&ctx.http, format!("✅ Unsubscribed from YouTube channel `{yt_id}`.")).await?;
            } else {
                msg.reply(&ctx.http, format!("❌ Subscription for `{yt_id}` not found.")).await?;
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
                "ℹ️ Usage:\n• `!youtube add <channel_id> [#discord_channel] [@ping_role]`\n• `!youtube remove <channel_id>`\n• `!youtube list`",
            ).await?;
        }
    }

    Ok(())
}

/// Register slash commands in Discord API
pub async fn register_slash_commands(ctx: &Context) {
    let yt_cmd = CreateCommand::new("youtube")
        .description("Configure YouTube upload notifications")
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "add", "Subscribe to a YouTube channel")
                .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "youtube_channel_id", "YouTube Channel ID (e.g. UC...)").required(true))
                .add_sub_option(CreateCommandOption::new(CommandOptionType::Channel, "discord_channel", "Discord channel to post videos in").required(true))
                .add_sub_option(CreateCommandOption::new(CommandOptionType::Role, "ping_role", "Optional role to mention").required(false))
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "remove", "Unsubscribe from a YouTube channel")
                .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "youtube_channel_id", "YouTube Channel ID").required(true))
        )
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "list", "List active YouTube subscriptions"));

    if let Err(e) = Command::set_global_commands(&ctx.http, vec![yt_cmd]).await {
        tracing::error!("Failed to register YouTube slash command: {e}");
    }
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
                let mut yt_id = None;
                let mut target_ch = None;
                let mut ping_role = None;

                if let CommandDataOptionValue::SubCommand(ref sub_opts) = sub.value {
                    for o in sub_opts {
                        match &o.value {
                            CommandDataOptionValue::String(s) => yt_id = Some(s.as_str()),
                            CommandDataOptionValue::Channel(c) => target_ch = Some(*c),
                            CommandDataOptionValue::Role(r) => ping_role = Some(r.get() as i64),
                            _ => {}
                        }
                    }
                }

                if let (Some(yt), Some(ch)) = (yt_id, target_ch) {
                    let _ = db::add_youtube_sub(&bot_data.db, guild_id.get() as i64, yt, ch.get() as i64, ping_role).await;
                    let role_str = ping_role.map(|r| format!(" (pinging <@&{r}>)")).unwrap_or_default();
                    let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(format!("✅ Subscribed to YouTube channel `{yt}` in <#{ch}>{role_str}.")).ephemeral(true)
                    )).await;
                } else {
                    let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content("❌ Please provide a valid YouTube Channel ID and Discord channel.").ephemeral(true)
                    )).await;
                }
            }
            "remove" => {
                let yt_id = if let CommandDataOptionValue::SubCommand(ref sub_opts) = sub.value {
                    sub_opts.iter().find_map(|o| match &o.value {
                        CommandDataOptionValue::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                } else {
                    None
                };

                if let Some(yt) = yt_id {
                    let removed = db::remove_youtube_sub(&bot_data.db, guild_id.get() as i64, yt).await.unwrap_or(false);
                    let resp = if removed {
                        format!("✅ Unsubscribed from YouTube channel `{yt}`.")
                    } else {
                        format!("❌ Subscription for `{yt}` not found.")
                    };
                    let _ = command.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(resp).ephemeral(true)
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
