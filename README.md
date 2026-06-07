# Tech News Agent

A Rust CLI tool that fetches the latest tech news from multiple communities, classifies and summarizes them, then pushes to WeChat channels.

## Features

- **Multi-source aggregation**: HackerNews, GitHub Trending, Rust Blog, Dev.to
- **Keyword-based classification**: AI/ML, Rust, Web, Backend, DevOps, Security, Open Source
- **WeChat push**: ServerChan, WxPusher, WeCom Bot
- **Cron scheduling**: Automatically fetch and push at configurable intervals

## Quick Start

```bash
cp .env.example .env
# Edit .env with your push channel config
cargo run
```

## Configuration

All settings are loaded from `.env`:

| Variable | Description | Default |
|---|---|---|
| `SERVERCHAN_KEY` | ServerChan push key | - |
| `WXPUSHER_TOKEN` | WxPusher app token | - |
| `WXPUSHER_UIDS` | WxPusher user IDs (comma-separated) | - |
| `WECOM_WEBHOOK` | WeCom bot webhook URL | - |
| `FETCH_INTERVAL` | Fetch interval in minutes | 60 |
| `MAX_ITEMS_PER_SOURCE` | Max items per source | 10 |
| `MIN_SCORE` | HackerNews min score filter | 100 |
| `GITHUB_LANG` | GitHub languages (comma-separated) | rust,golang,java,python,typescript |

At least one push channel must be configured.

## Architecture

```
src/
├── main.rs           # Entry point
├── config.rs         # Env config
├── models.rs          # Data structures
├── agent.rs           # Core coordinator
├── summarizer.rs     # Classification + Markdown rendering
├── scheduler.rs      # Cron scheduler
├── sources/          # News fetchers
│   ├── hackernews.rs
│   ├── github.rs
│   ├── rust_blog.rs
│   └── dev_to.rs
└── pusher/            # WeChat push channels
    ├── serverchan.rs
    ├── wxpusher.rs
    └── wecom_bot.rs
```
