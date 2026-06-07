以下是为 Claude Code 等 AI 编程助手量身定制的项目实现提示词 Markdown 文件。你可以直接将其保存为 `PROMPT.md` 并在项目中使用。
---
```markdown
# 🤖 Tech News Agent - AI 开发提示词
你是一个资深的 Rust 工程师。你的任务是根据以下规范，从零开始实现一个名为 **Tech News Agent** 的 Rust 命令行工具。
该工具能够：**并发抓取多个技术社区的最新资讯，智能分类汇总，并推送到微信**。
请严格按照本文件中的架构设计、数据结构和模块划分进行实现，不要遗漏任何模块。
---
## 1. 技术栈与依赖
- **语言版本**: Rust Edition 2021
- **异步运行时**: `tokio` (features = ["full"])
- **HTTP 客户端**: `reqwest` (features = ["json"])
- **序列化**: `serde` (features = ["derive"]), `serde_json`
- **时间处理**: `chrono` (features = ["serde"])
- **定时任务**: `tokio-cron-scheduler`
- **日志**: `tracing`, `tracing-subscriber` (features = ["env-filter"])
- **环境变量**: `dotenvy`
- **错误处理**: `thiserror`
- **异步 Trait**: `async-trait`
- **HTML 转文本**: `html2text`
请生成完整的 `Cargo.toml` 文件。
---
## 2. 项目目录结构
请严格遵循以下目录结构生成代码：
```text
tech-news-agent/
├── Cargo.toml
├── .env.example
├── src/
│   ├── main.rs          # 入口：加载配置、初始化日志、启动 Agent
│   ├── config.rs        # 从环境变量加载配置
│   ├── models.rs        # 核心数据结构 (NewsItem, Digest, Category, PushResult)
│   ├── agent.rs         # Agent 核心：协调抓取、汇总、推送
│   ├── summarizer.rs    # 分类引擎 + Markdown 渲染器
│   ├── scheduler.rs     # Cron 定时调度
│   ├── sources/
│   │   ├── mod.rs       # NewsSource trait 定义
│   │   ├── hackernews.rs # HackerNews API 抓取
│   │   ├── github.rs    # GitHub Search API 抓取
│   │   ├── rust_blog.rs # Rust Blog RSS 抓取
│   │   └── dev_to.rs    # Dev.to API 抓取
│   └── pusher/
│       ├── mod.rs       # Pusher trait 定义
│       ├── serverchan.rs # Server酱推送
│       ├── wxpusher.rs  # WxPusher推送
│       └── wecom_bot.rs # 企业微信机器人推送
```
---
## 3. 核心数据结构 (`src/models.rs`)
必须包含以下结构体和枚举，字段不可删减：
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub title: String,
    pub url: String,
    pub source: SourceKind,
    pub score: i64,
    pub summary: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub author: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SourceKind {
    HackerNews,
    GitHub,
    RustBlog,
    DevTo,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Digest {
    pub generated_at: DateTime<Utc>,
    pub total_items: usize,
    pub categories: Vec<Category>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    pub emoji: String,
    pub items: Vec<NewsItem>,
}
#[derive(Debug)]
pub struct PushResult {
    pub success: bool,
    pub message: String,
}
```
---
## 4. 配置管理 (`src/config.rs`)
实现 `AppConfig` 结构体，从 `.env` 读取以下配置：
- `SERVERCHAN_KEY`: Option<String>
- `WXPUSHER_TOKEN`: Option<String>
- `WXPUSHER_UIDS`: 逗号分隔，解析为 Vec<String>
- `WECOM_WEBHOOK`: Option<String>
- `FETCH_INTERVAL`: u64，默认 60 (分钟)
- `MAX_ITEMS_PER_SOURCE`: usize，默认 10
- `MIN_SCORE`: i64，默认 100 (HackerNews 最低票数)
- `GITHUB_LANG`: 逗号分隔，解析为 Vec<String>，默认 ["rust", "python", "typescript"]
提供 `has_push_channel(&self) -> bool` 方法验证至少配置了一种推送方式。
---
## 5. 信息源抓取 (`src/sources/`)
### 5.1 Trait 定义 (`mod.rs`)
```rust
#[async_trait]
pub trait NewsSource: Send + Sync {
    fn name(&self) -> &str;
    async fn fetch(&self, config: &AppConfig) -> Result<Vec<NewsItem>, String>;
}
```
### 5.2 HackerNews (`hackernews.rs`)
- **API 1**: `GET https://hacker-news.firebaseio.com/v0/topstories.json` 获取 Top Story IDs
- **API 2**: `GET https://hacker-news.firebaseio.com/v0/item/{id}.json` 获取详情
- **逻辑**: 取前 30 个 ID，并发请求详情；过滤掉 `url` 为 None 的 (Ask HN)；过滤掉 `score < config.min_hn_score` 的；取前 `config.max_items_per_source` 条
- **映射**: `score` -> `score`, `by` -> `author`, `time` (unix timestamp) -> `published_at`
### 5.3 GitHub Trending (`github.rs`)
- **API**: `GET https://api.github.com/search/repositories?q=language:{lang}+created:>{last_week}&sort=stars&order=desc`
- **逻辑**: 遍历 `config.github_langs`，分别请求；合并结果后按 `stargazers_count` 降序排序；取 Top N
- **映射**: `full_name` -> `title` (格式: "⭐ owner/repo (stars)"), `stargazers_count` -> `score`, `description` -> `summary`, `language` + `topics` -> `tags`
- **注意**: Request Header 需包含 `User-Agent` 和 `Accept: application/vnd.github.v3+json`
### 5.4 Rust Blog (`rust_blog.rs`)
- **API**: `GET https://api.rss2json.com/v1/api.json?rss_url=https://blog.rust-lang.org/feed.xml` (使用 rss2json 免费 API 将 RSS 转 JSON)
- **逻辑**: 获取最新文章，截取前 200 字符作为摘要
- **映射**: `tags` 固定包含 `["rust"]`
### 5.5 Dev.to (`dev_to.rs`)
- **API**: `GET https://dev.to/api/articles?per_page={max_items}&top=7` (获取过去 7 天热门)
- **映射**: `positive_reactions_count` -> `score`, `description` -> `summary`, `tag_list` (逗号分隔字符串) -> `tags`, `user.name` -> `author`
---
## 6. 智能汇总引擎 (`src/summarizer.rs`)
### 6.1 关键词分类
实现 `classify_by_keywords(title: &str, tags: &[String]) -> Vec<String>`，包含以下分类规则：
- "AI/ML": ai, ml, llm, gpt, machine learning, deep learning, transformer, claude, openai
- "Rust": rust, cargo, tokio, wasm, webassembly
- "Web": javascript, typescript, react, vue, nextjs, css, frontend, node
- "后端": database, postgres, redis, kafka, grpc, microservice, backend, api
- "DevOps": kubernetes, docker, k8s, terraform, ci/cd, devops, cloud
- "安全": security, vulnerability, cve, exploit, encryption
- "开源": open source, oss, github
- 默认: "其他技术"
### 6.2 Emoji 映射
AI/ML=🤖, Rust=🦀, Web=🌐, 后端=🗄️, DevOps=☁️, 安全=🔒, 开源=📦, 其他=💡
### 6.3 生成摘要
实现 `generate_digest(items: Vec<NewsItem>) -> Digest`:
1. 按 `classify_by_keywords` 将 items 分组到不同 Category
2. 每个 Category 内按 `score` 降序排序
3. Category 之间按 items 数量降序排序
### 6.4 Markdown 渲染
实现两个渲染函数：
- `render_markdown(digest: &Digest) -> String`: 完整版 Markdown，包含标题、链接、摘要、标签，适用于 Server酱 和 WxPusher
- `render_brief_markdown(digest: &Digest) -> String`: 精简版，每个分类只展示前 5 条标题，适用于企业微信机器人 (其 Markdown 不支持复杂格式)
---
## 7. 微信推送 (`src/pusher/`)
### 7.1 Trait 定义 (`mod.rs`)
```rust
#[async_trait]
pub trait Pusher: Send + Sync {
    fn name(&self) -> &str;
    async fn push(&self, digest: &Digest, content: &str) -> Result<PushResult, String>;
}
```
### 7.2 Server酱 (`serverchan.rs`)
- **构造**: `ServerChanPusher::new(config: &AppConfig) -> Option<Self>`，如果 `config.serverchan_key` 存在则创建
- **API**: `POST https://sctapi.ftqq.com/{key}.send`
- **Body**: Form 格式 `title=xxx&desp=xxx` (desp 支持 Markdown)
- **成功判断**: 返回 JSON 中 `code == 0`
### 7.3 WxPusher (`wxpusher.rs`)
- **构造**: 需要 `wxpusher_token` 和 `wxpusher_uids` 同时存在
- **API**: `POST https://wxpusher.zjiecode.com/api/send/message`
- **Body**: JSON 格式 `{ "app_token": "...", "content": "...", "summary": "...", "content_type": 3, "uids": [...] }` (content_type=3 表示 Markdown)
- **成功判断**: 返回 JSON 中 `code == 1000`
### 7.4 企业微信机器人 (`wecom_bot.rs`)
- **构造**: 需要 `wecom_webhook` 存在
- **API**: `POST {webhook_url}`
- **Body**: JSON 格式 `{ "msgtype": "markdown", "markdown": { "content": "..." } }`
- **注意**: 企业微信 Markdown 格式受限，只支持粗体和链接，必须使用 `render_brief_markdown` 生成的内容
- **成功判断**: 返回 JSON 中 `errcode == 0`
---
## 8. Agent 核心协调器 (`src/agent.rs`)
实现 `TechNewsAgent` 结构体：
```rust
pub struct TechNewsAgent {
    config: Arc<AppConfig>,
    sources: Vec<Box<dyn NewsSource>>,
    pushers: Vec<Box<dyn Pusher>>,
}
```
### 方法：
1. `new(config: AppConfig) -> Self`: 初始化所有 Sources 和 Pushers 实例
2. `run_once(&self)`: 执行一次完整流程
   - 调用 `fetch_all_sources()` 抓取
   - 如果结果为空，打印 warn 并 return
   - 调用 `generate_digest()` 汇总
   - 调用 `render_markdown()` 渲染
   - 遍历所有 pushers，调用 `push()` 推送，并记录成功/失败日志
3. `fetch_all_sources(&self) -> Vec<NewsItem>`: 遍历所有 sources 调用 `fetch()`，合并结果，基于 URL 去重
---
## 9. 定时调度器 (`src/scheduler.rs`)
实现 `start_scheduler(agent: Arc<TechNewsAgent>, interval_minutes: u64) -> Result<(), String>`:
- 使用 `tokio_cron_scheduler`
- Cron 表达式: `0 */{interval_minutes} * * * *`
- 异步任务中调用 `agent.run_once().await`
---
## 10. 程序入口 (`src/main.rs`)
流程：
1. `dotenvy::dotenv().ok()` 加载 .env
2. 初始化 `tracing` 日志 (Level::INFO)
3. `AppConfig::from_env()` 加载配置
4. 检查 `has_push_channel()`，如果为 false 则报错退出
5. 打印配置信息日志
6. 创建 `Arc::new(TechNewsAgent::new(config.clone()))`
7. 立即执行一次 `agent.run_once().await`
8. 启动调度器 `start_scheduler(agent, config.fetch_interval_minutes).await`
9. 使用 `tokio::signal::ctrl_c().await` 保持运行
---
## 11. 环境变量模板 (`.env.example`)
生成包含所有配置项及注释的 `.env.example` 文件。
---
## 12. 编码规范
- 使用 `tracing::info!`, `warn!`, `error!` 替代 `println!` 进行日志输出
- 所有错误使用 `Result<_, String>` 传递，使用 `.map_err(|e| format!("上下文: {e}"))?` 添加上下文
- HTTP Client 统一使用 `reqwest::Client::builder().user_agent("TechNewsAgent/0.1").build()` 构建
- 异步 Trait 必须使用 `#[async_trait]` 宏
- 注意 Rust 的所有权规则，在需要跨线程共享时使用 `Arc`
请现在开始实现，从 `Cargo.toml` 和 `.env.example` 开始，然后按模块顺序逐个生成代码。
```
---
将此文件保存后，你可以这样使用：
```bash
# 在 Claude Code 中
claude "请阅读 PROMPT.md，并按照规范逐步实现整个项目"
# 或在 Cursor/GitHub Copilot Chat 中
# 直接 @PROMPT.md 然后输入："请按此规范生成所有代码"
```
**提示词设计要点说明**：
| 设计维度 | 说明 |
|---------|------|
| **精确的数据结构** | 给出完整的 struct 定义，避免 AI 自行发挥导致字段不一致 |
| **API 端点明确** | 每个 Source 都指定了具体的 URL 和请求方式 |
| **业务逻辑细节** | 如 HN 的过滤条件、GitHub 的日期计算、摘要截取长度等 |
| **推送渠道差异** | 明确指出企业微信 Markdown 受限需用精简版 |
| **模块间契约** | Trait 签名固定，确保 Source/Agent/Pusher 之间接口一致 |
| **编码规范约束** | 统一错误处理、日志、HTTP Client 构建方式 |
