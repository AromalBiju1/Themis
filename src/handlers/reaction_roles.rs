use serenity::all::*;
use crate::{BotData, db};

pub async fn handle_reaction_add(ctx: &Context, reaction: &Reaction) {
    let guild_id = match reaction.guild_id {
        Some(g) => g,
        None => return,
    };
    let user_id = match reaction.user_id {
        Some(u) => u,
        None => return,
    };
    if user_id == ctx.cache.current_user().id {
        return;
    }

    let emoji_str = reaction.emoji.to_string();
    let pool = {
        let data = ctx.data.read().await;
        data.get::<BotData>().unwrap().db.clone()
    };

    if let Ok(Some(role_id)) = db::get_reaction_role(&pool, guild_id.get() as i64, reaction.message_id.get() as i64, &emoji_str).await {
        let role = RoleId::new(role_id as u64);
        if let Err(e) = ctx.http.add_member_role(guild_id, user_id, role, Some("Reaction role")).await {
            tracing::error!("reaction_roles: failed to add role: {e}");
        }
    }
}

pub async fn handle_reaction_remove(ctx: &Context, reaction: &Reaction) {
    let guild_id = match reaction.guild_id {
        Some(g) => g,
        None => return,
    };
    let user_id = match reaction.user_id {
        Some(u) => u,
        None => return,
    };

    let emoji_str = reaction.emoji.to_string();
    let pool = {
        let data = ctx.data.read().await;
        data.get::<BotData>().unwrap().db.clone()
    };

    if let Ok(Some(role_id)) = db::get_reaction_role(&pool, guild_id.get() as i64, reaction.message_id.get() as i64, &emoji_str).await {
        let role = RoleId::new(role_id as u64);
        if let Err(e) = ctx.http.remove_member_role(guild_id, user_id, role, Some("Reaction role removed")).await {
            tracing::error!("reaction_roles: failed to remove role: {e}");
        }
    }
}
