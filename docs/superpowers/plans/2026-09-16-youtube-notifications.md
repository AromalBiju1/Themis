# YouTube Video Notifications Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a zero-quota YouTube video notification system that receives WebSub push webhooks and fallback RSS feed polling to post new YouTube video uploads to Discord channels.

**Architecture:** WebSub push webhook (`/webhooks/youtube`) on Axum + background Tokio RSS poller with `If-None-Match` ETags + SQLite storage (`youtube_subscriptions`) + Serenity command handlers (`/youtube`, `$youtube`).

**Tech Stack:** Rust, Serenity 0.12, Axum 0.7, Sqlx (SQLite), Tokio, Reqwest.

## Global Constraints
- Database table name: `youtube_subscriptions`
- Command names: `/youtube` (Slash) and `$youtube` (Prefix)
- Support subcommands: `add`, `remove`, `list`

---

### Task 1: Database Schema & Dependency Setup

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/db.rs:1-60`

**Interfaces:**
- Produces: `struct YoutubeSub`, `db::init_db` updated to create `youtube_subscriptions` table.

- [ ] **Step 1: Update Cargo.toml**
Ensure `reqwest` with `json` feature is included in `Cargo.toml` dependencies if not already present.

- [ ] **Step 2: Add `youtube_subscriptions` table to `src/db.rs`**
Add `CREATE TABLE IF NOT EXISTS youtube_subscriptions` in `db::init_db`.

- [ ] **Step 3: Run cargo check**
Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit**
```bash
git add Cargo.toml src/db.rs
git commit -m "feat(youtube): add youtube_subscriptions database table schema"
```

---

### Task 2: Database CRUD Operations for YouTube Subscriptions

**Files:**
- Modify: `src/db.rs`

**Interfaces:**
- Consumes: `SqlitePool`
- Produces: `add_youtube_sub`, `remove_youtube_sub`, `list_youtube_subs`, `get_all_youtube_subs`, `update_youtube_last_video`

- [ ] **Step 1: Write YoutubeSub struct and DB helper functions in `src/db.rs`**
Implement `add_youtube_sub`, `remove_youtube_sub`, `list_youtube_subs`, `get_all_youtube_subs`, `update_youtube_last_video`.

- [ ] **Step 2: Run cargo check**
Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Commit**
```bash
git add src/db.rs
git commit -m "feat(youtube): add youtube database CRUD helper functions"
```

---

### Task 3: Atom XML Parser & Notification Embed Generator

**Files:**
- Create: `src/handlers/youtube.rs`

**Interfaces:**
- Consumes: `Context`, `YoutubeSub`, XML string input
- Produces: `parse_youtube_atom_feed(xml: &str) -> Option<YoutubeVideoInfo>`, `send_youtube_notification(...)`

- [ ] **Step 1: Implement `parse_youtube_atom_feed` and `send_youtube_notification`**
In `src/handlers/youtube.rs`, create regex/string-based parser for Atom `<entry>` tags (`video_id`, `title`, `channel_name`) and `send_youtube_notification` embed constructor.

- [ ] **Step 2: Write unit test for XML parsing**
Add `test_parse_youtube_atom_feed` unit test in `src/handlers/youtube.rs`.

- [ ] **Step 3: Run tests**
Run: `cargo test`
Expected: PASS (`test_parse_youtube_atom_feed ... ok`)

- [ ] **Step 4: Commit**
```bash
git add src/handlers/youtube.rs
git commit -m "feat(youtube): add Atom feed parser and notification embed generator"
```

---

### Task 4: WebSub Webhook Endpoint & RSS Poller

**Files:**
- Modify: `src/handlers/youtube.rs`

**Interfaces:**
- Consumes: `SqlitePool`, `Http` / `Context`
- Produces: `youtube_webhooks_router(pool: SqlitePool, http: Arc<Http>) -> Router`, `start_youtube_poller(pool: SqlitePool, http: Arc<Http>)`

- [ ] **Step 1: Implement Axum Webhook Handler**
Implement `GET` `/webhooks/youtube` (hub.challenge response) and `POST` `/webhooks/youtube` (Atom entry payload handler).

- [ ] **Step 2: Implement background Tokio poller**
Implement `start_youtube_poller` that loops every 5 minutes fetching RSS feeds with `If-None-Match` ETags.

- [ ] **Step 3: Run cargo check**
Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit**
```bash
git add src/handlers/youtube.rs
git commit -m "feat(youtube): add WebSub webhook server and background RSS poller"
```

---

### Task 5: YouTube Command Handlers (`/youtube` & `$youtube`)

**Files:**
- Create: `src/handlers/commands/youtube.rs`
- Modify: `src/handlers/commands/mod.rs`

**Interfaces:**
- Consumes: `Context`, `Message`, `Interaction`
- Produces: `register_slash_commands`, `handle_interaction`, `youtube` command group.

- [ ] **Step 1: Implement prefix and slash command handlers**
Create `src/handlers/commands/youtube.rs` with `add`, `remove`, and `list` subcommands.

- [ ] **Step 2: Export `youtube` in `src/handlers/commands/mod.rs`**
Add `pub mod youtube;` to `src/handlers/commands/mod.rs`.

- [ ] **Step 3: Run cargo check**
Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit**
```bash
git add src/handlers/commands/youtube.rs src/handlers/commands/mod.rs
git commit -m "feat(youtube): add /youtube slash and $youtube prefix command handlers"
```

---

### Task 6: Integration in `src/main.rs`

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `youtube::start_youtube_poller`, `youtube::youtube_webhooks_router`, `YOUTUBECMDS_GROUP`

- [ ] **Step 1: Wire YouTube commands & health/webhook router in `src/main.rs`**
Add `YOUTUBECMDS_GROUP` to framework, register slash commands in `ready`, route `/webhooks/youtube` in Axum router, and spawn `start_youtube_poller`.

- [ ] **Step 2: Run full build and test suite**
Run: `cargo test && cargo check`
Expected: PASS with 0 errors.

- [ ] **Step 3: Commit and push**
```bash
git add src/main.rs
git commit -m "feat(main): wire YouTube video notifications system"
git push origin master
```
