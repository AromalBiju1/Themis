use serenity::all::*;
use crate::BotData;

/// Called from the top-level EventHandler for every join event.
pub async fn handle_member_join(ctx: &Context, member: &Member) {
    let (autorole_id, guild_id) = {
        let data = ctx.data.read().await;
        let cfg = &data.get::<BotData>().unwrap().config;
        (cfg.autorole_id, member.guild_id)
    };

    if autorole_id == 0 {
        return;
    }

    let role_id = RoleId::new(autorole_id);

    // Verify the role exists in the guild cache (no await)
    let exists = ctx.cache
        .guild(guild_id)
        .map(|g| g.roles.contains_key(&role_id))
        .unwrap_or(false);

    if !exists {
        tracing::warn!("autorole: AUTOROLE_ID {autorole_id} not found in guild");
        return;
    }

    if let Err(e) = ctx.http
        .add_member_role(guild_id, member.user.id, role_id, Some("Autorole on join"))
        .await
    {
        tracing::error!("autorole: failed to add role: {e}");
    }
}
