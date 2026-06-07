use crate::config::AppConfig;
use crate::models::{Digest, PushResult};
use crate::pusher::Pusher;
use async_trait::async_trait;

pub struct ServerChanPusher {
    client: reqwest::Client,
    key: String,
}

impl ServerChanPusher {
    pub fn new(config: &AppConfig) -> Option<Self> {
        config.serverchan_key.as_ref().map(|key| Self {
            client: reqwest::Client::builder()
                .user_agent("TechNewsAgent/0.1")
                .build()
                .expect("build serverchan client"),
            key: key.clone(),
        })
    }
}

#[async_trait]
impl Pusher for ServerChanPusher {
    fn name(&self) -> &str {
        "ServerChan"
    }

    async fn push(&self, digest: &Digest, content: &str) -> Result<PushResult, String> {
        let url = format!("https://sctapi.ftqq.com/{}.send", self.key);
        let title = format!(
            "\u{1f4f0} Tech News Digest ({} items)",
            digest.total_items
        );

        let resp: serde_json::Value = self
            .client
            .post(&url)
            .form(&[("title", title.as_str()), ("desp", content)])
            .send()
            .await
            .map_err(|e| format!("ServerChan send: {e}"))?
            .json()
            .await
            .map_err(|e| format!("ServerChan parse: {e}"))?;

        let success = resp["code"].as_i64() == Some(0);
        let message = resp["message"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        Ok(PushResult { success, message })
    }
}
