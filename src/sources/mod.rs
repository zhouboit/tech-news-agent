pub mod arxiv;
pub mod dev_to;
pub mod security_advisory;
pub mod github;
pub mod hackernews;
pub mod rust_blog;

use crate::config::AppConfig;
use crate::models::NewsItem;
use async_trait::async_trait;

#[async_trait]
pub trait NewsSource: Send + Sync {
    fn name(&self) -> &str;
    async fn fetch(&self, config: &AppConfig) -> Result<Vec<NewsItem>, String>;
}
