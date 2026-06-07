use crate::config::AppConfig;
use crate::models::{Digest, PushResult};
use crate::pusher::Pusher;
use crate::summarizer::render_brief_markdown;
use async_trait::async_trait;
use serde_json::json;

pub struct WeComBotPusher {
    client: reqwest::Client,
    webhook_url: String,
}

impl WeComBotPusher {
    pub fn new(config: &AppConfig) -> Option<Self> {
        config.wecom_webhook.as_ref().map(|url| Self {
            client: reqwest::Client::builder()
                .user_agent("TechNewsAgent/0.1")
                .build()
                .expect("build wecom client"),
            webhook_url: url.clone(),
        })
    }
}

#[async_trait]
impl Pusher for WeComBotPusher {
    fn name(&self) -> &str {
        "WeComBot"
    }

    async fn push(&self, digest: &Digest, _content: &str) -> Result<PushResult, String> {
        let brief = render_brief_markdown(digest);
        let body = json!({
            "msgtype": "markdown",
            "markdown": { "content": brief }
        });

        let resp: serde_json::Value = self
            .client
            .post(&self.webhook_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("WeComBot send: {e}"))?
            .json()
            .await
            .map_err(|e| format!("WeComBot parse: {e}"))?;

        let success = resp["errcode"].as_i64() == Some(0);
        let message = resp["errmsg"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        Ok(PushResult { success, message })
    }
}
