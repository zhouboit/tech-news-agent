# Tech News Agent

Rust CLI tool: fetch tech news from HackerNews/GitHub/RustBlog/DevTo, classify, summarize, push to WeChat.

## Build & Run

```bash
cargo build
cargo run
```

## Project Structure

- `src/models.rs` — NewsItem, Digest, Category, PushResult, SourceKind
- `src/config.rs` — AppConfig from .env via dotenvy
- `src/sources/` — NewsSource trait + 4 fetchers (HN, GitHub, RustBlog, DevTo)
- `src/pusher/` — Pusher trait + 3 channels (ServerChan, WxPusher, WeCom Bot)
- `src/summarizer.rs` — keyword classification, digest generation, markdown rendering
- `src/agent.rs` — TechNewsAgent coordinator (concurrent fetch, dedup, digest, push)
- `src/scheduler.rs` — tokio-cron-scheduler wrapper

## Code Conventions

- Use `tracing::{info!, warn!, error!}` not `println!`
- Errors: `Result<_, String>`, `.map_err(|e| format!("context: {e}"))?`
- HTTP client: `reqwest::Client::builder().user_agent("TechNewsAgent/0.1").build()`
- Async traits: `#[async_trait]` macro
- Shared ownership: `Arc` where needed

## Key Details

- GitHub source fetches repos from last 3 days
- WxPusher API uses camelCase field names (`appToken`, `contentType`)
- Dev.to `tag_list` is a JSON array not a comma-separated string
- WeCom Bot uses `render_brief_markdown` (limited markdown support)
