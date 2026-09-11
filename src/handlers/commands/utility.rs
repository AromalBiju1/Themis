use serenity::{
    all::*,
    framework::standard::{macros::{command, group}, Args, CommandResult},
};
use crate::utils::check_user_mod;

#[group]
#[commands(userinfo, serverinfo, avatar, roleinfo, lock, unlock)]
pub struct UtilityCmds;

#[command]
#[description = "Display user details, join date, account creation date, and roles."]
pub async fn userinfo(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let target_user = if let Ok(parsed_id) = crate::utils::parse_target_from_args(&mut args).map(UserId::new) {
        ctx.http.get_user(parsed_id).await.unwrap_or_else(|_| msg.author.clone())
    } else {
        msg.author.clone()
    };

    let guild_id = msg.guild_id.unwrap_or_default();
    let member = ctx.http.get_member(guild_id, target_user.id).await.ok();

    let created_at = format!("<t:{}:F>", target_user.created_at().unix_timestamp());
    let joined_at = member.as_ref().and_then(|m| m.joined_at).map(|t| format!("<t:{}:F>", t.unix_timestamp())).unwrap_or_else(|| "Unknown".to_string());

    let roles_str = member.as_ref().map(|m| {
        let r: Vec<String> = m.roles.iter().map(|rid| format!("<@&{}>", rid)).collect();
        if r.is_empty() { "None".to_string() } else { r.join(", ") }
    }).unwrap_or_else(|| "None".to_string());

    let embed = CreateEmbed::new()
        .title(format!("👤 User Info: {}", target_user.tag()))
        .color(Color::BLURPLE)
        .thumbnail(target_user.face())
        .field("Mention", format!("<@{}>", target_user.id), true)
        .field("ID", target_user.id.to_string(), true)
        .field("Created At", created_at, false)
        .field("Joined Server", joined_at, false)
        .field("Roles", roles_str, false);

    msg.channel_id.send_message(&ctx.http, CreateMessage::new().embed(embed)).await?;
    Ok(())
}

#[command]
#[description = "Display server analytics, member count, creation date, and boost level."]
pub async fn serverinfo(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return Ok(()),
    };

    let guild = match ctx.http.get_guild(guild_id).await {
        Ok(g) => g,
        Err(_) => return Ok(()),
    };

    let created_at = format!("<t:{}:F>", guild.id.created_at().unix_timestamp());
    let owner_mention = format!("<@{}>", guild.owner_id);

    let embed = CreateEmbed::new()
        .title(format!("🏰 Server Info: {}", guild.name))
        .color(Color::GOLD)
        .thumbnail(guild.icon_url().unwrap_or_default())
        .field("Owner", owner_mention, true)
        .field("Members", guild.approximate_member_count.unwrap_or(0).to_string(), true)
        .field("Boost Tier", format!("{:?}", guild.premium_tier), true)
        .field("Created At", created_at, false);

    msg.channel_id.send_message(&ctx.http, CreateMessage::new().embed(embed)).await?;
    Ok(())
}

#[command]
#[description = "Display high-res user avatar."]
pub async fn avatar(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let target_user = if let Ok(parsed_id) = crate::utils::parse_target_from_args(&mut args).map(UserId::new) {
        ctx.http.get_user(parsed_id).await.unwrap_or_else(|_| msg.author.clone())
    } else {
        msg.author.clone()
    };

    let avatar_url = target_user.face();
    let embed = CreateEmbed::new()
        .title(format!("🖼️ Avatar: {}", target_user.tag()))
        .color(Color::LIGHT_GREY)
        .image(&avatar_url)
        .description(format!("[Direct Link]({})", avatar_url));

    msg.channel_id.send_message(&ctx.http, CreateMessage::new().embed(embed)).await?;
    Ok(())
}

#[command]
#[description = "Display role information and member count."]
pub async fn roleinfo(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let role_id = match args.single::<RoleId>() {
        Ok(r) => r,
        Err(_) => { msg.reply(&ctx.http, "❌ Provide a role mention or ID.").await?; return Ok(()); }
    };

    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return Ok(()),
    };

    if let Ok(guild) = ctx.http.get_guild(guild_id).await {
        if let Some(role) = guild.roles.get(&role_id) {
            let embed = CreateEmbed::new()
                .title(format!("🏷️ Role Info: {}", role.name))
                .color(role.colour)
                .field("ID", role.id.to_string(), true)
                .field("Color", format!("#{:06X}", role.colour.0), true)
                .field("Position", role.position.to_string(), true)
                .field("Mentionable", role.mentionable.to_string(), true)
                .field("Hoisted", role.hoist.to_string(), true);

            msg.channel_id.send_message(&ctx.http, CreateMessage::new().embed(embed)).await?;
            return Ok(());
        }
    }

    msg.reply(&ctx.http, "❌ Role not found.").await?;
    Ok(())
}

#[command]
#[description = "Lock down a text channel by revoking SendMessages for @everyone."]
pub async fn lock(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let cfg = &data.get::<crate::BotData>().unwrap().config;

    if !check_user_mod(ctx, msg, cfg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }

    let target_ch = args.single::<ChannelId>().unwrap_or(msg.channel_id);
    let reason = args.rest().trim();
    let reason_str = if reason.is_empty() { "Channel locked by moderator" } else { reason };

    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return Ok(()),
    };

    let everyone_role_id = RoleId::new(guild_id.get());
    let overwrite = PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::SEND_MESSAGES,
        kind: PermissionOverwriteType::Role(everyone_role_id),
    };

    if let Err(e) = target_ch.create_permission(&ctx.http, overwrite).await {
        msg.reply(&ctx.http, format!("❌ Lock failed: {e}")).await?;
    } else {
        msg.reply(&ctx.http, format!("🔒 Locked <#{}>. Reason: {}", target_ch, reason_str)).await?;
    }

    Ok(())
}

#[command]
#[description = "Unlock a text channel by restoring SendMessages for @everyone."]
pub async fn unlock(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let cfg = &data.get::<crate::BotData>().unwrap().config;

    if !check_user_mod(ctx, msg, cfg).await {
        msg.reply(&ctx.http, "❌ No permission.").await?;
        return Ok(());
    }

    let target_ch = args.single::<ChannelId>().unwrap_or(msg.channel_id);
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return Ok(()),
    };

    let everyone_role_id = RoleId::new(guild_id.get());
    let overwrite = PermissionOverwrite {
        allow: Permissions::SEND_MESSAGES,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Role(everyone_role_id),
    };

    if let Err(e) = target_ch.create_permission(&ctx.http, overwrite).await {
        msg.reply(&ctx.http, format!("❌ Unlock failed: {e}")).await?;
    } else {
        msg.reply(&ctx.http, format!("🔓 Unlocked <#{}>.", target_ch)).await?;
    }

    Ok(())
}
