use sqlx::{SqlitePool, Row};

/// Initialise the SQLite database and ensure the warns table exists.
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
    Ok(())
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
