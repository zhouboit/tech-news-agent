use crate::config::AppConfig;
use crate::models::{NewsItem, SourceKind};
use crate::sources::NewsSource;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::info;

#[derive(Deserialize)]
struct HnStory {
    title: Option<String>,
    url: Option<String>,
    score: Option<i64>,
    by: Option<String>,
    time: Option<i64>,
}

pub struct HackerNewsSource {
    client: reqwest::Client,
}

impl HackerNewsSource {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("TechNewsAgent/0.1")
            .build()
            .expect("build HN client");
        Self { client }
    }
}

#[async_trait]
impl NewsSource for HackerNewsSource {
    fn name(&self) -> &str {
        "HackerNews"
    }

    async fn fetch(&self, config: &AppConfig) -> Result<Vec<NewsItem>, String> {
        let top_ids: Vec<i64> = self
            .client
            .get("https://hacker-news.firebaseio.com/v0/topstories.json")
            .send()
            .await
            .map_err(|e| format!("HN fetch top stories: {e}"))?
            .json()
            .await
            .map_err(|e| format!("HN parse top stories: {e}"))?;

        let ids = &top_ids[..30.min(top_ids.len())];
        info!("HN: fetching {} stories", ids.len());

        let mut handles = Vec::new();
        for &id in ids {
            let client = self.client.clone();
            handles.push(tokio::spawn(async move {
                let url = format!("https://hacker-news.firebaseio.com/v0/item/{id}.json");
                let story: HnStory = client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| format!("HN fetch item {id}: {e}"))?
                    .json()
                    .await
                    .map_err(|e| format!("HN parse item {id}: {e}"))?;
                Ok::<HnStory, String>(story)
            }));
        }

        let mut items = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(story)) => {
                    if story.url.is_none() || story.score.unwrap_or(0) < config.min_score {
                        continue;
                    }
                    items.push(NewsItem {
                        title: story.title.unwrap_or_default(),
                        url: story.url.unwrap_or_default(),
                        source: SourceKind::HackerNews,
                        score: story.score.unwrap_or(0),
                        summary: None,
                        published_at: story.time.map(|t| DateTime::<Utc>::from_timestamp(t, 0).unwrap()),
                        tags: vec![],
                        author: story.by,
                        ai_analysis: None,
                    });
                }
                Ok(Err(e)) => {
                    tracing::warn!("HN: {e}");
                }
                Err(e) => {
                    tracing::warn!("HN task join error: {e}");
                }
            }
        }

        items.sort_by(|a, b| b.score.cmp(&a.score));
        items.truncate(config.max_items_per_source);
        info!("HN: got {} items after filtering", items.len());
        Ok(items)
    }
}
