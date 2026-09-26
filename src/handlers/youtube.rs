use serenity::all::*;
use axum::{extract::Query, routing::get, Router};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};
use serde::Deserialize;
use crate::db::{self, YoutubeSub};

#[derive(Debug, Clone, PartialEq)]
pub struct YoutubeVideoInfo {
    pub video_id:     String,
    pub title:        String,
    pub channel_name: String,
    pub channel_id:   String,
    pub published:    String,
}

pub fn unescape_xml(s: &str) -> String {
    s.replace("&amp;", "&")
     .replace("&lt;", "<")
     .replace("&gt;", ">")
     .replace("&quot;", "\"")
     .replace("&#39;", "'")
     .replace("&apos;", "'")
}

fn extract_tag_value(xml: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{tag}>");
    let close_tag = format!("</{tag}>");
    let start = xml.find(&open_tag)? + open_tag.len();
    let end = xml[start..].find(&close_tag)?;
    let val = &xml[start..start + end];
    Some(unescape_xml(val.trim()))
}

pub fn parse_all_youtube_entries(xml: &str) -> Vec<YoutubeVideoInfo> {
    let mut entries = Vec::new();
    let mut remaining = xml;

    while let Some(start) = remaining.find("<entry>") {
        let after_start = &remaining[start + 7..];
        let end = match after_start.find("</entry>") {
            Some(e) => e,
            None => break,
        };
        let entry_xml = &after_start[..end];

        let video_id = extract_tag_value(entry_xml, "yt:videoId").or_else(|| {
            let id = extract_tag_value(entry_xml, "id")?;
            id.strip_prefix("yt:video:").map(|s| s.to_string())
        });

        if let Some(vid) = video_id {
            let title = extract_tag_value(entry_xml, "title")
                .unwrap_or_else(|| "New YouTube Video".to_string());
            let channel_name = extract_tag_value(entry_xml, "name")
                .or_else(|| extract_tag_value(xml, "title"))
                .unwrap_or_else(|| "YouTube Channel".to_string());
            let channel_id = extract_tag_value(entry_xml, "yt:channelId")
                .or_else(|| extract_tag_value(xml, "yt:channelId"))
                .unwrap_or_default();
            let published = extract_tag_value(entry_xml, "published").unwrap_or_default();

            entries.push(YoutubeVideoInfo {
                video_id: vid,
                title,
                channel_name,
                channel_id,
                published,
            });
        }

        remaining = &after_start[end + 8..];
    }

    entries
}

pub fn parse_youtube_atom_feed(xml: &str) -> Option<YoutubeVideoInfo> {
    parse_all_youtube_entries(xml).into_iter().next()
}

pub async fn send_youtube_notification(
    http: &Http,
    sub: &YoutubeSub,
    video: &YoutubeVideoInfo,
) -> anyhow::Result<()> {
    let channel_id = ChannelId::new(sub.discord_channel_id as u64);
    let video_url = format!("https://www.youtube.com/watch?v={}", video.video_id);

    // Send native link without custom embed so Discord embeds the interactive playable YouTube video player
    let content = if let Some(role_id) = sub.ping_role_id {
        if role_id > 0 {
            format!("<@&{role_id}> 🎥 **{}** uploaded a new video!\n{video_url}", video.channel_name)
        } else {
            format!("🎥 **{}** uploaded a new video!\n{video_url}", video.channel_name)
        }
    } else {
        format!("🎥 **{}** uploaded a new video!\n{video_url}", video.channel_name)
    };

    let message = CreateMessage::new().content(content);
    channel_id.send_message(http, message).await?;
    Ok(())
}

// ── WebSub Webhook Endpoint Handler ──────────────────────────────────────────

#[derive(Deserialize)]
pub struct WebSubChallenge {
    #[serde(rename = "hub.challenge")]
    pub challenge: Option<String>,
}

async fn handle_websub_get(Query(params): Query<WebSubChallenge>) -> String {
    params.challenge.unwrap_or_default()
}

async fn handle_websub_post(
    axum::extract::State((pool, http)): axum::extract::State<(sqlx::SqlitePool, Arc<Http>)>,
    body: String,
) -> &'static str {
    if let Some(video) = parse_youtube_atom_feed(&body) {
        if !video.channel_id.is_empty() {
            if let Ok(subs) = db::get_all_youtube_subs(&pool).await {
                for sub in subs {
                    if sub.youtube_channel_id == video.channel_id {
                        let already_seen = db::is_youtube_video_seen(&pool, sub.guild_id, &sub.youtube_channel_id, &video.video_id).await.unwrap_or(false);
                        if !already_seen {
                            let _ = db::mark_youtube_video_seen(&pool, sub.guild_id, &sub.youtube_channel_id, &video.video_id).await;
                            let _ = db::update_youtube_sub_last_video(&pool, sub.id, &video.video_id, None).await;
                            let _ = send_youtube_notification(&http, &sub, &video).await;
                        }
                    }
                }
            }
        }
    }
    "OK"
}

pub fn youtube_webhooks_router(pool: sqlx::SqlitePool, http: Arc<Http>) -> Router {
    Router::new()
        .route("/webhooks/youtube", get(handle_websub_get).post(handle_websub_post))
        .with_state((pool, http))
}

fn extract_uc_channel_id(html: &str) -> Option<String> {
    let patterns = [
        "/channel/UC",
        "channel_id=UC",
        "itemprop=\"identifier\" content=\"UC",
        "\"channelId\":\"UC",
        "\"externalId\":\"UC",
        "\"browseId\":\"UC",
    ];

    for pat in patterns {
        if let Some(idx) = html.find(pat) {
            let start = idx + pat.len() - 2;
            if html.len() >= start + 24 {
                let candidate = &html[start..start + 24];
                if candidate.starts_with("UC") && candidate.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                    return Some(candidate.to_string());
                }
            }
        }
    }
    None
}

pub async fn resolve_youtube_channel(input: &str) -> anyhow::Result<(String, String, Option<YoutubeVideoInfo>)> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(10))
        .build()?;

    let trimmed = input.trim();
    
    // Extract handle or channel ID if full URL passed
    let clean_input = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        if let Some(idx) = trimmed.find("/channel/") {
            &trimmed[idx + 9..]
        } else if let Some(idx) = trimmed.find("/@") {
            &trimmed[idx + 1..]
        } else if let Some(idx) = trimmed.find("/c/") {
            &trimmed[idx + 3..]
        } else if let Some(idx) = trimmed.find("/user/") {
            &trimmed[idx + 6..]
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    let clean_input = clean_input.split('/').next().unwrap_or(clean_input).split('?').next().unwrap_or(clean_input);

    // 1. If clean_input is already a 24-char UC... channel ID
    if clean_input.starts_with("UC") && clean_input.len() == 24 {
        let rss_url = format!("https://www.youtube.com/feeds/videos.xml?channel_id={clean_input}");
        if let Ok(resp) = client.get(&rss_url).send().await {
            if resp.status().is_success() {
                if let Ok(xml) = resp.text().await {
                    if let Some(video) = parse_youtube_atom_feed(&xml) {
                        return Ok((clean_input.to_string(), video.channel_name.clone(), Some(video)));
                    }
                }
            }
        }
    }

    // 2. Try handle or channel page fetch
    let handle_name = if clean_input.starts_with('@') {
        clean_input.to_string()
    } else {
        format!("@{clean_input}")
    };

    let urls_to_try = [
        format!("https://www.youtube.com/{handle_name}"),
        format!("https://www.youtube.com/c/{clean_input}"),
        format!("https://www.youtube.com/user/{clean_input}"),
    ];

    for page_url in urls_to_try {
        if let Ok(resp) = client.get(&page_url).send().await {
            if let Ok(html) = resp.text().await {
                if let Some(ch_id) = extract_uc_channel_id(&html) {
                    let rss_url = format!("https://www.youtube.com/feeds/videos.xml?channel_id={ch_id}");
                    if let Ok(rss_resp) = client.get(&rss_url).send().await {
                        if let Ok(xml) = rss_resp.text().await {
                            if let Some(video) = parse_youtube_atom_feed(&xml) {
                                return Ok((ch_id, video.channel_name.clone(), Some(video)));
                            }
                        }
                    }
                    return Ok((ch_id, clean_input.to_string(), None));
                }
            }
        }
    }

    Err(anyhow::anyhow!("Could not resolve YouTube Channel ID for '{input}'. Please check the YouTube handle or channel link."))
}

// ── Background Tokio Poller (Fallback) ────────────────────────────────────────

pub fn start_youtube_poller(pool: sqlx::SqlitePool, http: Arc<Http>) {
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        loop {
            let subs = match db::get_all_youtube_subs(&pool).await {
                Ok(s) => s,
                Err(e) => {
                    warn!("youtube_poller: failed to fetch subscriptions: {e}");
                    sleep(Duration::from_secs(120)).await;
                    continue;
                }
            };

            for sub in subs {
                let mut channel_id = sub.youtube_channel_id.clone();

                // Auto-migrate old raw handle / URL records in SQLite to true UC channel ID
                if !(channel_id.starts_with("UC") && channel_id.len() == 24) {
                    info!("youtube_poller: found un-migrated sub id {} ('{}'), auto-resolving...", sub.id, channel_id);
                    match resolve_youtube_channel(&channel_id).await {
                        Ok((resolved_id, _name, _)) => {
                            info!("youtube_poller: successfully auto-migrated sub id {} ('{}') -> '{}'", sub.id, channel_id, resolved_id);
                            let _ = db::update_youtube_sub_channel_id(&pool, sub.id, &resolved_id).await;
                            channel_id = resolved_id.clone();
                        }
                        Err(e) => {
                            warn!("youtube_poller: failed to auto-resolve YouTube channel for sub id {} ('{}'): {e}", sub.id, channel_id);
                            continue;
                        }
                    }
                }

                let url = format!(
                    "https://www.youtube.com/feeds/videos.xml?channel_id={}",
                    channel_id
                );

                let resp = match client.get(&url).send().await {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("youtube_poller: request error for {}: {e}", channel_id);
                        continue;
                    }
                };

                if !resp.status().is_success() {
                    continue;
                }

                let xml_text = match resp.text().await {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                let entries = parse_all_youtube_entries(&xml_text);
                if entries.is_empty() {
                    continue;
                }

                let seen_count = db::get_seen_video_count(&pool, sub.guild_id, &channel_id).await.unwrap_or(0);

                if seen_count == 0 {
                    // Initial baseline: mark all current videos as seen so past videos and future edits to past videos NEVER ping!
                    info!("youtube_poller: seeding initial seen videos for channel {}", channel_id);
                    for entry in &entries {
                        let _ = db::mark_youtube_video_seen(&pool, sub.guild_id, &channel_id, &entry.video_id).await;
                    }
                    if let Some(newest) = entries.first() {
                        let _ = db::update_youtube_sub_last_video(&pool, sub.id, &newest.video_id, None).await;
                    }
                    continue;
                }

                // Check entries from oldest to newest so notifications arrive in chronological order
                let mut reversed = entries;
                reversed.reverse();

                for video in reversed {
                    let already_seen = db::is_youtube_video_seen(&pool, sub.guild_id, &channel_id, &video.video_id).await.unwrap_or(false);
                    if already_seen {
                        // Already seen! Even if creator edits title, thumbnail, or description 100 times, NEVER PING!
                        continue;
                    }

                    // Genuinely NEW video upload!
                    info!("youtube_poller: new upload detected '{}' ({}) for {}", video.title, video.video_id, channel_id);
                    let mut updated_sub = sub.clone();
                    updated_sub.youtube_channel_id = channel_id.clone();
                    let _ = send_youtube_notification(&http, &updated_sub, &video).await;
                    let _ = db::mark_youtube_video_seen(&pool, sub.guild_id, &channel_id, &video.video_id).await;
                    let _ = db::update_youtube_sub_last_video(&pool, sub.id, &video.video_id, None).await;
                }
            }

            sleep(Duration::from_secs(120)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_youtube_atom_feed() {
        let sample_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns:yt="http://www.youtube.com/xml/schemas/2015" xmlns="http://www.w3.org/2005/Atom">
  <link rel="self" href="https://www.youtube.com/feeds/videos.xml?channel_id=UC123456789"/>
  <id>yt:channel:UC123456789</id>
  <yt:channelId>UC123456789</yt:channelId>
  <title>Epic Gaming Channel</title>
  <entry>
    <id>yt:video:abc_XYZ123</id>
    <yt:videoId>abc_XYZ123</yt:videoId>
    <yt:channelId>UC123456789</yt:channelId>
    <title>Awesome New Gameplay &amp; Review!</title>
    <link rel="alternate" href="https://www.youtube.com/watch?v=abc_XYZ123"/>
    <author>
      <name>Epic Gaming Channel</name>
    </author>
  </entry>
</feed>"#;

        let parsed = parse_youtube_atom_feed(sample_xml).expect("Failed to parse sample XML");
        assert_eq!(parsed.video_id, "abc_XYZ123");
        assert_eq!(parsed.title, "Awesome New Gameplay & Review!");
        assert_eq!(parsed.channel_name, "Epic Gaming Channel");
        assert_eq!(parsed.channel_id, "UC123456789");
    }

    #[test]
    fn test_extract_uc_channel_id() {
        let html_rss = r#"<link rel="alternate" type="application/rss+xml" href="https://www.youtube.com/feeds/videos.xml?channel_id=UC1234567890123456789012">"#;
        assert_eq!(extract_uc_channel_id(html_rss), Some("UC1234567890123456789012".to_string()));

        let html_canonical = r#"<link rel="canonical" href="https://www.youtube.com/channel/UCabcdefghijklmnopqrstuv">"#;
        assert_eq!(extract_uc_channel_id(html_canonical), Some("UCabcdefghijklmnopqrstuv".to_string()));

        let html_meta = r#"<meta itemprop="identifier" content="UC9876543210987654321098">"#;
        assert_eq!(extract_uc_channel_id(html_meta), Some("UC9876543210987654321098".to_string()));
    }

    #[test]
    fn test_parse_all_youtube_entries() {
        let multi_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns:yt="http://www.youtube.com/xml/schemas/2015" xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <id>yt:video:vid1</id>
    <title>Video 1</title>
    <yt:channelId>UC111</yt:channelId>
    <published>2026-09-25T10:00:00+00:00</published>
  </entry>
  <entry>
    <id>yt:video:vid2</id>
    <title>Video 2</title>
    <yt:channelId>UC111</yt:channelId>
    <published>2026-09-26T10:00:00+00:00</published>
  </entry>
</feed>"#;
        let entries = parse_all_youtube_entries(multi_xml);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].video_id, "vid1");
        assert_eq!(entries[1].video_id, "vid2");
    }
}
