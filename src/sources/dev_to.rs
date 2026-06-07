use crate::config::AppConfig;
use crate::models::{NewsItem, SourceKind};
use crate::sources::NewsSource;
use async_trait::async_trait;
use serde::Deserialize;
use tracing::info;

#[derive(Deserialize)]
struct DevToArticle {
    title: String,
    url: String,
    positive_reactions_count: i64,
    description: Option<String>,
    tag_list: Option<Vec<String>>,
    user: Option<DevToUser>,
}

#[derive(Deserialize)]
struct DevToUser {
    name: Option<String>,
}

pub struct DevToSource {
    client: reqwest::Client,
}

impl DevToSource {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("TechNewsAgent/0.1")
            .build()
            .expect("build DevTo client");
        Self { client }
    }
}

#[async_trait]
impl NewsSource for DevToSource {
    fn name(&self) -> &str {
        "Dev.to"
    }

    async fn fetch(&self, config: &AppConfig) -> Result<Vec<NewsItem>, String> {
        let max_items = config.max_items_per_source;
        let url = format!("https://dev.to/api/articles?per_page={max_items}&top=7");
        let articles: Vec<DevToArticle> = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("DevTo fetch: {e}"))?
            .json()
            .await
            .map_err(|e| format!("DevTo parse: {e}"))?;

        let items: Vec<NewsItem> = articles
            .into_iter()
            .map(|a| {
                let tags = a
                    .tag_list
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|t| !t.is_empty())
                    .collect();
                NewsItem {
                    title: a.title,
                    url: a.url,
                    source: SourceKind::DevTo,
                    score: a.positive_reactions_count,
                    summary: a.description,
                    published_at: None,
                    tags,
                    author: a.user.and_then(|u| u.name),
                }
            })
            .collect();

        info!("DevTo: got {} items", items.len());
        Ok(items)
    }
}
