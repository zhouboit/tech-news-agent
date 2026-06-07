use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;

use chrono::Timelike;

use crate::config::AppConfig;
use crate::models::NewsItem;
use crate::pusher::{serverchan::ServerChanPusher, wecom_bot::WeComBotPusher, wxpusher::WxPusherPusher, Pusher};
use crate::sources::{arxiv::ArxivSource, dev_to::DevToSource, github::GitHubSource, hackernews::HackerNewsSource, rust_blog::RustBlogSource, security_advisory::SecurityAdvisorySource, NewsSource};
use crate::stock::StockQuote;
use crate::summarizer::{generate_digest, render_markdown};
use tracing::{info, warn};

pub struct TechNewsAgent {
    config: Arc<AppConfig>,
    sources: Vec<Arc<dyn NewsSource>>,
    pushers: Vec<Box<dyn Pusher>>,
    last_content_hash: Mutex<u64>,
    last_stock_hash: Mutex<u64>,
    last_seen_urls: Mutex<HashSet<String>>,
}

impl TechNewsAgent {
    pub fn new(config: AppConfig) -> Self {
        let sources: Vec<Arc<dyn NewsSource>> = vec![
            Arc::new(HackerNewsSource::new()),
            Arc::new(GitHubSource::new()),
            Arc::new(RustBlogSource::new()),
            Arc::new(DevToSource::new()),
            Arc::new(ArxivSource::new()),
            Arc::new(SecurityAdvisorySource::new()),
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
            last_content_hash: Mutex::new(0),
            last_stock_hash: Mutex::new(0),
            last_seen_urls: Mutex::new(HashSet::new()),
        }
    }

    pub async fn run_once(&self) {
        info!("Starting news fetch cycle...");

        let hour = chrono::Local::now().hour();
        let is_quiet_hours = hour >= 22 || hour < 6;
        if is_quiet_hours {
            info!("Quiet hours (22:00-06:00), skipping push");
            return;
        }

        let mut items = self.fetch_all_sources().await;
        if items.is_empty() {
            warn!("No news items fetched, skipping push");
            return;
        }

        let news_hash = hash_items(&items);
        let news_changed = {
            let mut last = self.last_content_hash.lock().unwrap();
            let changed = *last != news_hash;
            *last = news_hash;
            changed
        };

        if news_changed {
            info!("Total {} items fetched (content changed), running AI analysis...", items.len());
            crate::zhipu::analyze_items(&mut items, &self.config).await;
        } else {
            info!("Total {} items fetched (content unchanged), skipping AI analysis", items.len());
        }

        info!("Generating digest...");
        let current_urls: HashSet<String> = items.iter().map(|i| i.url.clone()).collect();
        let new_urls = {
            let mut seen = self.last_seen_urls.lock().unwrap();
            let new: HashSet<String> = current_urls.difference(&*seen).cloned().collect();
            *seen = current_urls;
            new
        };
        if !new_urls.is_empty() {
            info!("{} new items in this cycle", new_urls.len());
        }
        let digest = generate_digest(items, new_urls);
        let mut content = render_markdown(&digest);

        let stock_client = reqwest::Client::builder()
            .user_agent("TechNewsAgent/0.1")
            .build()
            .expect("build stock client");

        // Market indices (always fetched, not cached)
        let market_indices = crate::stock::fetch_market_indices(&stock_client).await;

        let mut stock_quotes = crate::stock::fetch_stocks(&self.config).await;
        let stock_hash = hash_stocks(&stock_quotes);
        let stock_changed = {
            let mut last = self.last_stock_hash.lock().unwrap();
            let changed = *last != stock_hash;
            *last = stock_hash;
            changed
        };

        if stock_changed {
            info!("Stock data changed, running stock AI analysis...");
            crate::stock::analyze_stocks_with_ai(&mut stock_quotes, &self.config).await;
        } else {
            info!("Stock data unchanged, skipping stock AI analysis");
        }

        let stock_news = crate::stock::fetch_stock_news(&stock_client, 5).await;

        // Policy section (before stock, so it's not truncated)
        let mut policy_news = crate::policy::fetch_policy_news(&stock_client).await;
        crate::policy::analyze_policies_with_ai(
            &mut policy_news,
            &self.config.stock_watch_list,
            &self.config,
        )
        .await;
        let policy_section = crate::policy::render_policy_section(&policy_news);
        if !policy_section.is_empty() {
            info!("Policy section: {} chars", policy_section.len());
            content = format!("{}\n---\n\n{}", policy_section, content);
        } else {
            info!("Policy section: empty (no analyzed policies)");
        }

        if !market_indices.is_empty() || !stock_quotes.is_empty() || !stock_news.is_empty() {
            let stock_section = crate::stock::render_stock_section(&market_indices, &stock_quotes, &stock_news);
            content = format!("{}\n---\n\n{}", stock_section, content);
        }

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
        let mut seen_urls = HashSet::new();

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

fn hash_items(items: &[NewsItem]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    for item in items {
        item.title.hash(&mut hasher);
        item.url.hash(&mut hasher);
    }
    hasher.finish()
}

fn hash_stocks(quotes: &[StockQuote]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    for q in quotes {
        q.code.hash(&mut hasher);
        q.current.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}
