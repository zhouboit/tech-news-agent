mod agent;
mod config;
mod models;
mod pusher;
mod scheduler;
mod sources;
mod summarizer;

use std::sync::Arc;

use config::AppConfig;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    let config = AppConfig::from_env();

    if !config.has_push_channel() {
        tracing::error!("No push channel configured. Please set at least one of: SERVERCHAN_KEY, WXPUSHER_TOKEN+UIDS, WECOM_WEBHOOK");
        std::process::exit(1);
    }

    info!("Config: interval={}min, max_items={}, min_score={}, langs={:?}",
        config.fetch_interval_minutes,
        config.max_items_per_source,
        config.min_score,
        config.github_langs,
    );

    let agent = Arc::new(agent::TechNewsAgent::new(config.clone()));

    agent.run_once().await;

    info!("Starting scheduler...");
    if let Err(e) = scheduler::start_scheduler(agent, config.fetch_interval_minutes).await {
        tracing::error!("Scheduler error: {}", e);
    }
}
