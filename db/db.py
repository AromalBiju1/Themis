"""
SQLite persistence layer via aiosqlite.
Stores: warns
"""
import aiosqlite
from config.config import DB_PATH

async def init_db():
    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute("""
            CREATE TABLE IF NOT EXISTS warns (
                id        INTEGER PRIMARY KEY AUTOINCREMENT,
                guild_id  INTEGER NOT NULL,
                user_id   INTEGER NOT NULL,
                mod_id    INTEGER NOT NULL,
                reason    TEXT    NOT NULL,
                ts        TEXT    NOT NULL
            )
        """)
        await db.commit()

async def add_warn(guild_id, user_id, mod_id, reason, ts):
    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute(
            "INSERT INTO warns (guild_id, user_id, mod_id, reason, ts) VALUES (?,?,?,?,?)",
            (guild_id, user_id, mod_id, reason, ts)
        )
        await db.commit()

async def get_warns(guild_id, user_id):
    async with aiosqlite.connect(DB_PATH) as db:
        async with db.execute(
            "SELECT id, mod_id, reason, ts FROM warns WHERE guild_id=? AND user_id=? ORDER BY id",
            (guild_id, user_id)
        ) as cur:
            return await cur.fetchall()

async def clear_warns(guild_id, user_id):
    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute("DELETE FROM warns WHERE guild_id=? AND user_id=?", (guild_id, user_id))
        await db.commit()

async def remove_warn(warn_id):
    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute("DELETE FROM warns WHERE id=?", (warn_id,))
        await db.commit()