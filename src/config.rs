use std::env;
use serenity::all::*;

/// Loaded once at startup from environment variables / .env
#[derive(Debug, Clone)]
pub struct Config {
    pub discord_token:     String,
    pub guild_id:          u64,
    pub mod_log_channel:   u64,
    pub immune_role_ids:   Vec<u64>,
    pub mod_role_ids:      Vec<u64>,

    // Honeypot
    pub honeypot_channel:  u64,

    // Antispam
    pub spam_msg_limit:    usize,
    pub spam_window_secs:  u64,
    pub anti_invite:       bool,
    pub raid_join_limit:   usize,
    pub raid_window_secs:  u64,

    // Warns
    pub warn_mute_at:      u32,
    pub warn_kick_at:      u32,
    pub warn_ban_at:       u32,
    pub mute_role_id:      u64,

    // Autorole
    pub autorole_id:       u64,

    // DB
    pub db_path:           String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Self {
            discord_token:    require("DISCORD_TOKEN")?,
            guild_id:         parse_int("GUILD_ID", 0),
            mod_log_channel:  parse_int("MOD_LOG_CHANNEL_ID", 0),
            immune_role_ids:  parse_set("IMMUNE_ROLE_IDS"),
            mod_role_ids:     parse_set("MOD_ROLE_IDS"),

            honeypot_channel: parse_int("HONEYPOT_CHANNEL_ID", 0),

            spam_msg_limit:   parse_int("SPAM_MSG_LIMIT", 5)   as usize,
            spam_window_secs: parse_int("SPAM_WINDOW_SECS", 5),
            anti_invite:      parse_bool("ANTI_INVITE", true),
            raid_join_limit:  parse_int("RAID_JOIN_LIMIT", 10) as usize,
            raid_window_secs: parse_int("RAID_WINDOW_SECS", 10),

            warn_mute_at:     parse_int("WARN_MUTE_AT", 3)     as u32,
            warn_kick_at:     parse_int("WARN_KICK_AT", 5)     as u32,
            warn_ban_at:      parse_int("WARN_BAN_AT", 7)      as u32,
            mute_role_id:     parse_int("MUTE_ROLE_ID", 0),

            autorole_id:      parse_int("AUTOROLE_ID", 0),

            db_path:          env::var("DB_PATH").unwrap_or_else(|_| "modbot.db".to_string()),
        })
    }

    pub fn is_immune(&self, member: &Member) -> bool {
        if member.user.bot {
            return true;
        }
        if member.permissions.map(|p| p.administrator()).unwrap_or(false) {
            return true;
        }
        member.roles.iter().any(|r| self.immune_role_ids.contains(&r.get()))
    }

    pub fn is_mod(&self, member: &Member) -> bool {
        if member.permissions.map(|p| p.administrator()).unwrap_or(false) {
            return true;
        }
        member.roles.iter().any(|r| self.mod_role_ids.contains(&r.get()))
    }
}

fn require(key: &str) -> anyhow::Result<String> {
    env::var(key).map_err(|_| anyhow::anyhow!("Missing required env var: {}", key))
}

fn parse_int<T: std::str::FromStr + Default>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

fn parse_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .map(|v| matches!(v.to_lowercase().trim(), "1" | "true" | "yes"))
        .unwrap_or(default)
}

fn parse_set(key: &str) -> Vec<u64> {
    env::var(key)
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| s.trim().parse::<u64>().ok())
        .collect()
}
