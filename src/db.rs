use sqlx::{SqlitePool, Row};

/// Initialise the SQLite database and ensure the tables exist.
pub async fn init_db(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS warns (
            id       INTEGER PRIMARY KEY AUTOINCREMENT,
            guild_id INTEGER NOT NULL,
            user_id  INTEGER NOT NULL,
            mod_id   INTEGER NOT NULL,
            reason   TEXT    NOT NULL,
            ts       TEXT    NOT NULL
        )"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS welcome_config (
            guild_id   INTEGER PRIMARY KEY,
            channel_id INTEGER NOT NULL,
            title      TEXT,
            message    TEXT NOT NULL,
            image_url  TEXT,
            enabled    INTEGER NOT NULL DEFAULT 1
        )"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS reaction_roles (
            guild_id   INTEGER NOT NULL,
            channel_id INTEGER NOT NULL,
            message_id INTEGER NOT NULL,
            emoji      TEXT    NOT NULL,
            role_id    INTEGER NOT NULL,
            PRIMARY KEY (guild_id, message_id, emoji)
        )"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS goodbye_config (
            guild_id   INTEGER PRIMARY KEY,
            channel_id INTEGER NOT NULL,
            title      TEXT,
            message    TEXT NOT NULL,
            image_url  TEXT,
            enabled    INTEGER NOT NULL DEFAULT 1
        )"
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn add_reaction_role(
    pool: &SqlitePool,
    guild_id: i64,
    channel_id: i64,
    message_id: i64,
    emoji: &str,
    role_id: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO reaction_roles (guild_id, channel_id, message_id, emoji, role_id) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(guild_id, message_id, emoji) DO UPDATE SET role_id=EXCLUDED.role_id"
    )
    .bind(guild_id)
    .bind(channel_id)
    .bind(message_id)
    .bind(emoji)
    .bind(role_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_reaction_role(
    pool: &SqlitePool,
    guild_id: i64,
    message_id: i64,
    emoji: &str,
) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM reaction_roles WHERE guild_id=? AND message_id=? AND emoji=?")
        .bind(guild_id)
        .bind(message_id)
        .bind(emoji)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_reaction_role(
    pool: &SqlitePool,
    guild_id: i64,
    message_id: i64,
    emoji: &str,
) -> anyhow::Result<Option<i64>> {
    let row = sqlx::query(
        "SELECT role_id FROM reaction_roles WHERE guild_id=? AND message_id=? AND emoji=?"
    )
    .bind(guild_id)
    .bind(message_id)
    .bind(emoji)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.get::<i64, _>("role_id")))
}

#[derive(Debug, Clone)]
pub struct GoodbyeConfig {
    pub guild_id: i64,
    pub channel_id: i64,
    pub title: Option<String>,
    pub message: String,
    pub image_url: Option<String>,
    pub enabled: bool,
}

pub async fn get_goodbye_config(pool: &SqlitePool, guild_id: i64) -> anyhow::Result<Option<GoodbyeConfig>> {
    let row = sqlx::query(
        "SELECT guild_id, channel_id, title, message, image_url, enabled FROM goodbye_config WHERE guild_id=?"
    )
    .bind(guild_id)
    .fetch_optional(pool)
    .await?;

    if let Some(r) = row {
        Ok(Some(GoodbyeConfig {
            guild_id: r.get("guild_id"),
            channel_id: r.get("channel_id"),
            title: r.get("title"),
            message: r.get("message"),
            image_url: r.get("image_url"),
            enabled: r.get::<i64, _>("enabled") != 0,
        }))
    } else {
        Ok(None)
    }
}

pub async fn set_goodbye_channel(pool: &SqlitePool, guild_id: i64, channel_id: i64) -> anyhow::Result<()> {
    let default_msg = "👋 Goodbye {user}! We hope to see you again soon in {server}.";
    sqlx::query(
        "INSERT INTO goodbye_config (guild_id, channel_id, message, enabled) VALUES (?, ?, ?, 1)
         ON CONFLICT(guild_id) DO UPDATE SET channel_id=EXCLUDED.channel_id"
    )
    .bind(guild_id)
    .bind(channel_id)
    .bind(default_msg)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_goodbye_text(pool: &SqlitePool, guild_id: i64, text: &str) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO goodbye_config (guild_id, channel_id, message, enabled) VALUES (?, 0, ?, 1)
         ON CONFLICT(guild_id) DO UPDATE SET message=EXCLUDED.message"
    )
    .bind(guild_id)
    .bind(text)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn toggle_goodbye(pool: &SqlitePool, guild_id: i64) -> anyhow::Result<bool> {
    let current = get_goodbye_config(pool, guild_id).await?.map(|c| c.enabled).unwrap_or(true);
    let new_val = if current { 0 } else { 1 };
    sqlx::query("UPDATE goodbye_config SET enabled=? WHERE guild_id=?")
        .bind(new_val)
        .bind(guild_id)
        .execute(pool)
        .await?;
    Ok(new_val == 1)
}

#[derive(Debug, Clone)]
pub struct WelcomeConfig {
    pub guild_id: i64,
    pub channel_id: i64,
    pub title: Option<String>,
    pub message: String,
    pub image_url: Option<String>,
    pub enabled: bool,
}

pub async fn get_welcome_config(pool: &SqlitePool, guild_id: i64) -> anyhow::Result<Option<WelcomeConfig>> {
    let row = sqlx::query(
        "SELECT guild_id, channel_id, title, message, image_url, enabled FROM welcome_config WHERE guild_id=?"
    )
    .bind(guild_id)
    .fetch_optional(pool)
    .await?;

    if let Some(r) = row {
        Ok(Some(WelcomeConfig {
            guild_id: r.get("guild_id"),
            channel_id: r.get("channel_id"),
            title: r.get("title"),
            message: r.get("message"),
            image_url: r.get("image_url"),
            enabled: r.get::<i64, _>("enabled") != 0,
        }))
    } else {
        Ok(None)
    }
}

pub async fn set_welcome_channel(pool: &SqlitePool, guild_id: i64, channel_id: i64) -> anyhow::Result<()> {
    let default_msg = "💠 • There is no rules so feel free to say anything .\n💠 • BTW don't say too much cringe things 👀\n\n💠 • If you have any more questions then dm one of our staff, mods, and admins.\n\n• ﾟ. 🌟 ﾟ. 🌟 Please Enjoy your Stay! 🌟 ﾟ. 🌟 ﾟ.";
    sqlx::query(
        "INSERT INTO welcome_config (guild_id, channel_id, message, enabled) VALUES (?, ?, ?, 1)
         ON CONFLICT(guild_id) DO UPDATE SET channel_id=EXCLUDED.channel_id"
    )
    .bind(guild_id)
    .bind(channel_id)
    .bind(default_msg)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_welcome_text(pool: &SqlitePool, guild_id: i64, text: &str) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO welcome_config (guild_id, channel_id, message, enabled) VALUES (?, 0, ?, 1)
         ON CONFLICT(guild_id) DO UPDATE SET message=EXCLUDED.message"
    )
    .bind(guild_id)
    .bind(text)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_welcome_image(pool: &SqlitePool, guild_id: i64, image_url: &str) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO welcome_config (guild_id, channel_id, message, image_url, enabled) VALUES (?, 0, '', ?, 1)
         ON CONFLICT(guild_id) DO UPDATE SET image_url=EXCLUDED.image_url"
    )
    .bind(guild_id)
    .bind(image_url)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn toggle_welcome(pool: &SqlitePool, guild_id: i64) -> anyhow::Result<bool> {
    let current = get_welcome_config(pool, guild_id).await?.map(|c| c.enabled).unwrap_or(true);
    let new_val = if current { 0 } else { 1 };
    sqlx::query(
        "UPDATE welcome_config SET enabled=? WHERE guild_id=?"
    )
    .bind(new_val)
    .bind(guild_id)
    .execute(pool)
    .await?;
    Ok(new_val == 1)
}

pub async fn add_warn(
    pool:     &SqlitePool,
    guild_id: i64,
    user_id:  i64,
    mod_id:   i64,
    reason:   &str,
    ts:       &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO warns (guild_id, user_id, mod_id, reason, ts) VALUES (?,?,?,?,?)"
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(mod_id)
    .bind(reason)
    .bind(ts)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug)]
pub struct WarnRow {
    pub id:     i64,
    pub mod_id: i64,
    pub reason: String,
    pub ts:     String,
}

pub async fn get_warns(
    pool:     &SqlitePool,
    guild_id: i64,
    user_id:  i64,
) -> anyhow::Result<Vec<WarnRow>> {
    let rows = sqlx::query(
        "SELECT id, mod_id, reason, ts FROM warns WHERE guild_id=? AND user_id=? ORDER BY id"
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| WarnRow {
            id:     r.get::<i64, _>("id"),
            mod_id: r.get::<i64, _>("mod_id"),
            reason: r.get::<String, _>("reason"),
            ts:     r.get::<String, _>("ts"),
        })
        .collect())
}

pub async fn count_warns(pool: &SqlitePool, guild_id: i64, user_id: i64) -> anyhow::Result<u32> {
    let row = sqlx::query(
        "SELECT COUNT(*) as cnt FROM warns WHERE guild_id=? AND user_id=?"
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row.get::<i64, _>("cnt") as u32)
}

pub async fn clear_warns(pool: &SqlitePool, guild_id: i64, user_id: i64) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM warns WHERE guild_id=? AND user_id=?")
        .bind(guild_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn remove_warn(pool: &SqlitePool, warn_id: i64) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM warns WHERE id=?")
        .bind(warn_id)
        .execute(pool)
        .await?;
    Ok(())
}
