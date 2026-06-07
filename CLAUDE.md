# Tech News Agent

Rust CLI: concurrent fetch 6 news sources + A-share stocks, Zhipu AI analysis, classify into 9 categories, intelligence insights, push to 3 WeChat channels.

## Build & Run

```bash
cargo build
cargo run
```

## Project Structure

- `src/models.rs` — NewsItem, AiAnalysis (brief_name, keywords, summary_cn, impact, action, prediction), Digest, Category, SourceKind (with `display_name()`), PushResult
- `src/config.rs` — AppConfig from .env via dotenvy (push channels, GitHub langs, Zhipu AI, stock watch list)
- `src/sources/` — NewsSource trait + 6 fetchers (HN, GitHub, RustBlog, DevTo, arXiv, SecurityAdvisory)
- `src/pusher/` — Pusher trait + 3 channels (ServerChan, WxPusher, WeCom Bot)
- `src/zhipu.rs` — Zhipu AI client: batch analysis 15 items/batch, JSON extraction via `[...]` slicing + item-level recovery
- `src/stock.rs` — Sina Finance quotes, Zhipu AI stock analysis, East Money hot news
- `src/summarizer.rs` — 9-category keyword classification, breakthrough/cross-domain detection, full/brief markdown (Chinese)
- `src/agent.rs` — TechNewsAgent: fetch -> AI analyze -> digest -> stock fetch+analyze -> stock news -> render -> push
- `src/scheduler.rs` — tokio-cron-scheduler wrapper

## Code Conventions

- `tracing::{info!, warn!, error!}` not `println!`
- Errors: `Result<_, String>`, `.map_err(|e| format!("context: {e}"))?`
- HTTP client: `reqwest::Client::builder().user_agent("TechNewsAgent/0.1").build()`
- Async traits: `#[async_trait]` macro
- Shared ownership: `Arc` for sources in spawned tasks
- All display text in Chinese

## Key Details

- GitHub source: repos from last 3 days
- arXiv source: cs.AI + cs.LG categories, Atom XML parsing via quick-xml, title whitespace cleanup
- WxPusher: camelCase fields (`appToken`, `contentType`), success code 1000
- Dev.to: `tag_list` is JSON array, not comma-separated
- WeCom Bot: uses `render_brief_markdown` (limited markdown support)
- Zhipu AI: split 50 items into batches of 15, extract JSON via `[...]` bounds, fallback to per-object `{...}` recovery
- Stock quotes: Sina Finance API, auto-detect sh/sz prefix by code (6/9->sh, else->sz)
- Quiet hours: skip push 22:00-06:00 (`chrono::Local::now().hour()`)
- Timezone: all timestamps displayed in local timezone via `with_timezone(&Local)`
