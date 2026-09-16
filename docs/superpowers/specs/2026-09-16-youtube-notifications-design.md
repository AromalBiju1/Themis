# YouTube Video Notifications System Design

## Goal
Implement a reliable, zero-quota-exhaustion YouTube video notification system for ThemisBot that posts new video uploads to designated Discord channels without hitting API rate limits or IP 429 throttling.

## Problem Statement
Existing Discord bots (e.g., Sapphire) rely on naive HTML scraping or frequent YouTube Data API polling. YouTube imposes a strict 10,000 quota units/day limit on API calls and aggressive 429 rate-limiting on IP scraping, causing third-party bots to get throttled or stop posting videos.

## Solution Architecture
ThemisBot combines **WebSub (PubSubHubbub) Push Webhooks** (primary) with an **ETag-cached RSS Feed Poller** (fallback) to ensure real-time delivery with zero API quota usage.

### 1. WebSub Push Webhook (`/webhooks/youtube`)
- Exposes an HTTP endpoint on Axum server (`:PORT/webhooks/youtube`).
- Handles `GET` challenge verification (`hub.challenge`) sent by Google's PubSubHubbub hub (`https://pubsubhubbub.appspot.com/subscribe`).
- Handles `POST` notification payloads containing Atom XML feed items for newly published videos.

### 2. ETag-Cached RSS Feed Poller (Background Task)
- Background Tokio task running every 5 minutes.
- Fetches `https://www.youtube.com/feeds/videos.xml?channel_id=CHANNEL_ID` for all active subscriptions using `reqwest`.
- Includes `If-None-Match` HTTP headers based on cached ETags. YouTube responds with `304 Not Modified` when no new videos are available, incurring zero bandwidth penalty and avoiding IP throttling.

### 3. Database Schema (`youtube_subscriptions` in `modbot.db`)
```sql
CREATE TABLE IF NOT EXISTS youtube_subscriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id INTEGER NOT NULL,
    youtube_channel_id TEXT NOT NULL,
    discord_channel_id INTEGER NOT NULL,
    ping_role_id INTEGER,
    last_video_id TEXT,
    etag TEXT,
    UNIQUE(guild_id, youtube_channel_id)
);
```

### 4. Commands (`/youtube` & `$youtube`)
- `add <youtube_channel_id> <discord_channel> [ping_role]` — Subscribe to a YouTube channel.
- `remove <youtube_channel_id>` — Unsubscribe from a YouTube channel.
- `list` — List all active subscriptions in the server.

### 5. Notification Message Format
- **Content**: Optional ping role mention (`<@&role_id>`) + video link (`https://youtu.be/<video_id>`).
- **Embed**:
  - Author: YouTube Channel Name
  - Title: Video Title (clickable link to YouTube video)
  - Color: Red (`#FF0000`)
  - Image/Thumbnail: High-res video thumbnail (`https://i.ytimg.com/vi/<video_id>/maxresdefault.jpg`)
  - Footer: `Uploaded to YouTube` + timestamp.

## Verification Plan
1. Unit tests for Atom XML RSS parsing.
2. Integration check for database subscription CRUD functions.
3. End-to-end `cargo test` and `cargo check`.
