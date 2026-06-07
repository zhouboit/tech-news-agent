# Tech News Agent

Rust CLI tool: concurrent fetch tech news & stock quotes, AI-powered analysis via Zhipu AI, classify, summarize, push to WeChat.

## Features

- **6 News Sources**: HackerNews, GitHub Trending (last 3 days), Rust Blog, Dev.to, arXiv (cs.AI + cs.LG), GitHub Security Advisory
- **Stock Market**: A-share real-time quotes (Sina Finance), 3 market indices (上证/深证/创业板), AI stock analysis, hot market news (East Money)
- **Policy & Regulation**: Multi-channel policy news (重要财经/社会政策/财经热点), AI-powered categorization (社会/经济/制造/技术), related A-share stock recommendations
- **AI Analysis**: Zhipu AI (GLM-4) batch analysis — Chinese brief name, keywords, summary, impact, action advice, trend prediction
- **Intelligent Digest**: Keyword classification (9 categories), breakthrough detection, cross-domain correlation insights, deduplication
- **Content Change Detection**: Hash-based caching skips redundant AI API calls when content is unchanged
- **3 Push Channels**: ServerChan, WxPusher, WeCom Bot (each with content summary detection)
- **Cron Scheduling**: Configurable interval with quiet hours (22:00-06:00)

## Quick Start

```bash
cp .env.example .env
# Edit .env: set at least one push channel + ZHIPU_API_KEY
cargo run
```

## Configuration

All settings in `.env`:

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
| `ZHIPU_API_KEY` | Zhipu AI API key | - |
| `ZHIPU_MODEL` | Zhipu AI model | glm-4 |
| `STOCK_WATCH_LIST` | A-share stock codes (comma-separated) | 688326,600967 |

At least one push channel must be configured. Zhipu AI is optional (falls back to raw data).

## Architecture

```
src/
├── main.rs              # Entry: load config, init logging, run agent, start scheduler
├── config.rs            # AppConfig from .env
├── models.rs            # NewsItem, AiAnalysis, Digest, Category, SourceKind, PushResult
├── agent.rs             # TechNewsAgent: concurrent fetch -> AI analysis -> digest -> stock/policy -> push
├── summarizer.rs        # Keyword classification, digest generation, full/brief markdown rendering
├── scheduler.rs         # tokio-cron-scheduler wrapper
├── zhipu.rs             # Zhipu AI client: batch analysis (15 items/batch), JSON recovery
├── stock.rs             # A-share quotes (Sina), market indices, AI stock analysis, hot news (East Money)
├── policy.rs             # Policy news (East Money multi-channel), AI categorization + stock recommendations
├── sources/             # NewsSource trait + fetchers
│   ├── hackernews.rs    #   Top stories API, score filter
│   ├── github.rs        #   Trending repos by language
│   ├── rust_blog.rs     #   RSS via rss2json
│   ├── dev_to.rs        #   Top articles API
│   ├── arxiv.rs         #   cs.AI + cs.LG papers (Atom XML)
│   └── security_advisory.rs  # GitHub Advisory API
└── pusher/              # Pusher trait + channels
    ├── serverchan.rs    #   Form POST
    ├── wxpusher.rs      #   JSON POST (camelCase fields, content summary)
    └── wecom_bot.rs     #   JSON POST (brief markdown, section extraction)
```

## Push Content Layout

```
📈 股市行情          # Market indices + watched stocks + AI analysis + hot news
---
📜 政策法规解读        # Policy news grouped by category (社会/经济/制造/技术) + stock recommendations
---
📰 技术资讯日报        # 9-category tech news with AI analysis per item
---
🔬 情报洞察           # Breakthrough detection + cross-domain correlations
```
