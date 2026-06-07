# Tech News Agent

Rust CLI: concurrent fetch 6 news sources + A-share stocks, policy news, Zhipu AI analysis, classify into 9 categories, intelligence insights, push to 3 WeChat channels.

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
- `src/stock.rs` — Sina Finance quotes (sh/sz prefix auto-detect via code), market indices (上证/深证/创业板), Zhipu AI stock analysis, East Money hot news (BK0473 channel)
- `src/policy.rs` — East Money multi-channel policy news (BK0425 重要财经, BK0520 社会政策, BK0475 财经热点), Zhipu AI categorization (社会/经济/制造/技术), related A-share stock recommendations
- `src/summarizer.rs` — 9-category keyword classification, breakthrough/cross-domain detection with deduplication, full/brief markdown (Chinese)
- `src/agent.rs` — TechNewsAgent: fetch -> AI analyze -> digest -> stock fetch+analyze -> policy fetch+analyze -> render -> push; content-change hash to skip redundant AI calls
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
- WxPusher: camelCase fields (`appToken`, `contentType`), success code 1000, summary detection for 📈股市/📜政策/🔬情报
- Dev.to: `tag_list` is JSON array, not comma-separated
- WeCom Bot: uses `render_brief_markdown` (limited markdown support), extracts stock/policy sections from full content
- Zhipu AI: split 50 items into batches of 15, extract JSON via `[...]` bounds, fallback to per-object `{...}` recovery
- Stock quotes: Sina Finance API `var hq_str_sh688326="..."`, auto-detect sh/sz prefix (6/9->sh, else->sz), `extract_sina_code` splits on `=`
- Stock news: East Money API requires `mTypeAndCode` param, response fields are `Art_Title`/`Art_Url`/`Art_ShowTime`
- Policy news: 3 East Money channels with title deduplication; `render_policy_section` shows items even without AI analysis
- Content change detection: `DefaultHasher` over news titles+urls and stock codes+prices, skips AI calls when unchanged
- Quiet hours: skip push 22:00-06:00 (`chrono::Local::now().hour()`)
- Timezone: all timestamps displayed in local timezone via `with_timezone(&Local)`
