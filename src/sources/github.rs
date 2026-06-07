use crate::config::AppConfig;
use crate::models::{NewsItem, SourceKind};
use crate::sources::NewsSource;
use async_trait::async_trait;
use serde::Deserialize;
use tracing::info;

#[derive(Deserialize)]
struct GhRepo {
    full_name: String,
    stargazers_count: i64,
    description: Option<String>,
    #[allow(dead_code)]
    language: Option<String>,
    topics: Vec<String>,
    html_url: String,
}

pub struct GitHubSource {
    client: reqwest::Client,
}

impl GitHubSource {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("TechNewsAgent/0.1")
            .build()
            .expect("build GitHub client");
        Self { client }
    }
}

#[async_trait]
impl NewsSource for GitHubSource {
    fn name(&self) -> &str {
        "GitHub"
    }

    async fn fetch(&self, config: &AppConfig) -> Result<Vec<NewsItem>, String> {
        let last_week = chrono::Utc::now() - chrono::Duration::days(3);
        let date_str = last_week.format("%Y-%m-%d").to_string();

        let mut all_items = Vec::new();
        for lang in &config.github_langs {
            let url = format!(
                "https://api.github.com/search/repositories?q=language:{lang}+created:>{date_str}&sort=stars&order=desc"
            );
            let resp: serde_json::Value = self
                .client
                .get(&url)
                .header("Accept", "application/vnd.github.v3+json")
                .send()
                .await
                .map_err(|e| format!("GitHub search {lang}: {e}"))?
                .json()
                .await
                .map_err(|e| format!("GitHub parse {lang}: {e}"))?;

            if let Some(items) = resp["items"].as_array() {
                for item in items {
                    let repo: GhRepo = match serde_json::from_value(item.clone()) {
                        Ok(r) => r,
                        Err(_) => continue,
                    };
                    let mut tags = vec![lang.clone()];
                    tags.extend(repo.topics);
                    all_items.push(NewsItem {
                        title: format!(
                            "\u{2b50} {} ({} stars)",
                            repo.full_name, repo.stargazers_count
                        ),
                        url: repo.html_url,
                        source: SourceKind::GitHub,
                        score: repo.stargazers_count,
                        summary: repo.description,
                        published_at: None,
                        tags,
                        author: None,
                        ai_analysis: None,
                    });
                }
            }
        }

        all_items.sort_by(|a, b| b.score.cmp(&a.score));
        all_items.truncate(config.max_items_per_source);
        info!("GitHub: got {} items", all_items.len());
        Ok(all_items)
    }
}
