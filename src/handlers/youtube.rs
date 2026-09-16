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

pub fn parse_youtube_atom_feed(xml: &str) -> Option<YoutubeVideoInfo> {
    let entry_start = xml.find("<entry>")?;
    let entry_xml = &xml[entry_start..];

    let video_id = extract_tag_value(entry_xml, "yt:videoId").or_else(|| {
        let id = extract_tag_value(entry_xml, "id")?;
        id.strip_prefix("yt:video:").map(|s| s.to_string())
    })?;

    let title = extract_tag_value(entry_xml, "title")
        .unwrap_or_else(|| "New YouTube Video".to_string());

    let channel_name = extract_tag_value(entry_xml, "name")
        .or_else(|| extract_tag_value(xml, "title"))
        .unwrap_or_else(|| "YouTube Channel".to_string());

    let channel_id = extract_tag_value(entry_xml, "yt:channelId")
        .or_else(|| extract_tag_value(xml, "yt:channelId"))
        .unwrap_or_default();

    Some(YoutubeVideoInfo {
        video_id,
        title,
        channel_name,
        channel_id,
    })
}

pub async fn send_youtube_notification(
    http: &Http,
    sub: &YoutubeSub,
    video: &YoutubeVideoInfo,
) -> anyhow::Result<()> {
    let channel_id = ChannelId::new(sub.discord_channel_id as u64);
    let video_url = format!("https://youtu.be/{}", video.video_id);
    let thumbnail_url = format!("https://i.ytimg.com/vi/{}/maxresdefault.jpg", video.video_id);

    let content = if let Some(role_id) = sub.ping_role_id {
        if role_id > 0 {
            format!("<@&{role_id}> 🎥 **New Video Uploaded!**\n{video_url}")
        } else {
            format!("🎥 **New Video Uploaded!**\n{video_url}")
        }
    } else {
        format!("🎥 **New Video Uploaded!**\n{video_url}")
    };

    let embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::new(&video.channel_name))
        .title(&video.title)
        .url(&video_url)
        .color(Color::from_rgb(255, 0, 0)) // Red YouTube theme
        .image(thumbnail_url)
        .footer(CreateEmbedFooter::new("Uploaded to YouTube"));

    let message = CreateMessage::new().content(content).embed(embed);
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
        if let Ok(subs) = db::get_all_youtube_subs(&pool).await {
            for sub in subs {
                if sub.youtube_channel_id == video.channel_id || video.channel_id.is_empty() {
                    if sub.last_video_id.as_deref() != Some(&video.video_id) {
                        let _ = db::update_youtube_sub_last_video(&pool, sub.id, &video.video_id, None).await;
                        let _ = send_youtube_notification(&http, &sub, &video).await;
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

// ── Background Tokio Poller (Fallback) ────────────────────────────────────────

pub fn start_youtube_poller(pool: sqlx::SqlitePool, http: Arc<Http>) {
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (ThemisBot YouTube Notifier)")
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        loop {
            sleep(Duration::from_secs(300)).await; // Poll every 5 minutes

            let subs = match db::get_all_youtube_subs(&pool).await {
                Ok(s) => s,
                Err(e) => {
                    warn!("youtube_poller: failed to fetch subscriptions: {e}");
                    continue;
                }
            };

            for sub in subs {
                let url = format!(
                    "https://www.youtube.com/feeds/videos.xml?channel_id={}",
                    sub.youtube_channel_id
                );

                let mut req = client.get(&url);
                if let Some(ref etag) = sub.etag {
                    req = req.header("If-None-Match", etag);
                }

                let resp = match req.send().await {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("youtube_poller: request error for {}: {e}", sub.youtube_channel_id);
                        continue;
                    }
                };

                if resp.status() == reqwest::StatusCode::NOT_MODIFIED {
                    // 304 Not Modified -> no new videos, zero bandwidth consumed
                    continue;
                }

                let new_etag = resp.headers().get("etag").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
                let xml_text = match resp.text().await {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                if let Some(video) = parse_youtube_atom_feed(&xml_text) {
                    if sub.last_video_id.as_deref() != Some(&video.video_id) {
                        info!("youtube_poller: new video found '{}' for {}", video.title, sub.youtube_channel_id);
                        let _ = db::update_youtube_sub_last_video(&pool, sub.id, &video.video_id, new_etag.as_deref()).await;
                        let _ = send_youtube_notification(&http, &sub, &video).await;
                    }
                }
            }
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
}
