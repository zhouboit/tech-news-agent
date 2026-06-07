use std::sync::Arc;

use crate::config::AppConfig;
use crate::models::NewsItem;
use crate::pusher::{serverchan::ServerChanPusher, wecom_bot::WeComBotPusher, wxpusher::WxPusherPusher, Pusher};
use crate::sources::{dev_to::DevToSource, github::GitHubSource, hackernews::HackerNewsSource, rust_blog::RustBlogSource, NewsSource};
use crate::summarizer::{generate_digest, render_markdown};
use tracing::{info, warn};

pub struct TechNewsAgent {
    config: Arc<AppConfig>,
    sources: Vec<Arc<dyn NewsSource>>,
    pushers: Vec<Box<dyn Pusher>>,
}

impl TechNewsAgent {
    pub fn new(config: AppConfig) -> Self {
        let sources: Vec<Arc<dyn NewsSource>> = vec![
            Arc::new(HackerNewsSource::new()),
            Arc::new(GitHubSource::new()),
            Arc::new(RustBlogSource::new()),
            Arc::new(DevToSource::new()),
        ];

        let mut pushers: Vec<Box<dyn Pusher>> = Vec::new();
        if let Some(p) = ServerChanPusher::new(&config) {
            pushers.push(Box::new(p));
        }
        if let Some(p) = WxPusherPusher::new(&config) {
            pushers.push(Box::new(p));
        }
        if let Some(p) = WeComBotPusher::new(&config) {
            pushers.push(Box::new(p));
        }

        info!("Agent: {} sources, {} pushers configured", sources.len(), pushers.len());
        Self {
            config: Arc::new(config),
            sources,
            pushers,
        }
    }

    pub async fn run_once(&self) {
        info!("Starting news fetch cycle...");

        let items = self.fetch_all_sources().await;
        if items.is_empty() {
            warn!("No news items fetched, skipping push");
            return;
        }

        info!("Total {} items fetched, generating digest...", items.len());
        let digest = generate_digest(items);
        let content = render_markdown(&digest);

        for pusher in &self.pushers {
            match pusher.push(&digest, &content).await {
                Ok(result) => {
                    if result.success {
                        info!("Push [{}]: success - {}", pusher.name(), result.message);
                    } else {
                        warn!("Push [{}]: failed - {}", pusher.name(), result.message);
                    }
                }
                Err(e) => {
                    tracing::error!("Push [{}] error: {}", pusher.name(), e);
                }
            }
        }
    }

    async fn fetch_all_sources(&self) -> Vec<NewsItem> {
        let mut handles = Vec::new();
        for source in &self.sources {
            let config = Arc::clone(&self.config);
            let source = Arc::clone(source);
            let name = source.name().to_string();
            handles.push(tokio::spawn(async move {
                let items = source.fetch(&config).await;
                (name, items)
            }));
        }

        let mut all_items = Vec::new();
        let mut seen_urls = std::collections::HashSet::new();

        for handle in handles {
            match handle.await {
                Ok((name, Ok(items))) => {
                    info!("Source [{}]: {} items", name, items.len());
                    for item in items {
                        if seen_urls.insert(item.url.clone()) {
                            all_items.push(item);
                        }
                    }
                }
                Ok((name, Err(e))) => {
                    warn!("Source [{}] error: {}", name, e);
                }
                Err(e) => {
                    warn!("Source task join error: {}", e);
                }
            }
        }

        all_items
    }
}
