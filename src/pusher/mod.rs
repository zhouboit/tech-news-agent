pub mod serverchan;
pub mod wecom_bot;
pub mod wxpusher;

use crate::models::{Digest, PushResult};
use async_trait::async_trait;

#[async_trait]
pub trait Pusher: Send + Sync {
    fn name(&self) -> &str;
    async fn push(&self, digest: &Digest, content: &str) -> Result<PushResult, String>;
}
