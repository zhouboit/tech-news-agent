use crate::config::AppConfig;
use crate::models::{NewsItem, SourceKind};
use crate::sources::NewsSource;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::info;

#[derive(Deserialize)]
struct RssResponse {
    items: Option<Vec<RssItem>>,
}

#[derive(Deserialize)]
struct RssItem {
    title: Option<String>,
    link: Option<String>,
    pub_date: Option<String>,
    description: Option<String>,
}

pub struct RustBlogSource {
    client: reqwest::Client,
}

impl RustBlogSource {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("TechNewsAgent/0.1")
            .build()
            .expect("build RustBlog client");
        Self { client }
    }
}

fn parse_rfc2822_date(s: &str) -> Option<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc2822(s)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
}

#[async_trait]
impl NewsSource for RustBlogSource {
    fn name(&self) -> &str {
        "RustBlog"
    }

    async fn fetch(&self, config: &AppConfig) -> Result<Vec<NewsItem>, String> {
        let url = "https://api.rss2json.com/v1/api.json?rss_url=https://blog.rust-lang.org/feed.xml";
        let resp: RssResponse = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("RustBlog fetch: {e}"))?
            .json()
            .await
            .map_err(|e| format!("RustBlog parse: {e}"))?;

        let mut items = Vec::new();
        let rss_items = resp.items.unwrap_or_default();
        for item in rss_items {
            let summary = item
                .description
                .as_deref()
                .map(|d| {
                    let text = html2text::from_read(d.as_bytes(), 200);
                    let trimmed: String = text.chars().take(200).collect();
                    trimmed
                })
                .or(item.description.clone());

            items.push(NewsItem {
                title: item.title.unwrap_or_default(),
                url: item.link.unwrap_or_default(),
                source: SourceKind::RustBlog,
                score: 0,
                summary,
                published_at: item.pub_date.as_deref().and_then(parse_rfc2822_date),
                tags: vec!["rust".to_string()],
                author: None,
            });
        }

        items.truncate(config.max_items_per_source);
        info!("RustBlog: got {} items", items.len());
        Ok(items)
    }
}
