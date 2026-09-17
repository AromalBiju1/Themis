pub mod modcmds;
pub mod warns;
pub mod welcome;
pub mod rr;
pub mod utility;
pub mod embed;
pub mod goodbye;
pub mod youtube;

use serenity::all::*;

/// Register all global slash commands together in a single API call so they don't overwrite each other
pub async fn register_all_slash_commands(ctx: &Context) {
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
            CreateCommandOption::new(CommandOptionType::SubCommand, "set_image", "Set the bottom banner image (upload file or URL)")
                .add_sub_option(CreateCommandOption::new(CommandOptionType::Attachment, "image", "Upload image file directly").required(false))
                .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "url", "Direct image URL").required(false))
        )
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "toggle", "Toggle welcome messages on/off"))
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "status", "View current welcome settings"));

    let test_cmd = CreateCommand::new("welcometest")
        .description("Send a test welcome message in the designated channel");

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

    let all_commands = vec![welcome_cmd, test_cmd, yt_cmd];

    if let Err(e) = Command::set_global_commands(&ctx.http, all_commands).await {
        tracing::error!("Failed to register global slash commands: {e}");
    } else {
        tracing::info!("Registered global slash commands: /welcome, /welcometest, /youtube");
    }
}
