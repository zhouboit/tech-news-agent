use crate::config::AppConfig;
use crate::models::{Digest, PushResult};
use crate::pusher::Pusher;
use async_trait::async_trait;
use serde_json::json;

pub struct WxPusherPusher {
    client: reqwest::Client,
    token: String,
    uids: Vec<String>,
}

impl WxPusherPusher {
    pub fn new(config: &AppConfig) -> Option<Self> {
        if config.wxpusher_token.is_none() || config.wxpusher_uids.is_empty() {
            return None;
        }
        Some(Self {
            client: reqwest::Client::builder()
                .user_agent("TechNewsAgent/0.1")
                .build()
                .expect("build wxpusher client"),
            token: config.wxpusher_token.clone().unwrap(),
            uids: config.wxpusher_uids.clone(),
        })
    }
}

#[async_trait]
impl Pusher for WxPusherPusher {
    fn name(&self) -> &str {
        "WxPusher"
    }

    async fn push(&self, digest: &Digest, content: &str) -> Result<PushResult, String> {
        let url = "https://wxpusher.zjiecode.com/api/send/message";
        let body = json!({
            "appToken": self.token,
            "content": content,
            "summary": format!("\u{1f4f0} Tech News ({} items)", digest.total_items),
            "contentType": 3,
            "uids": self.uids,
        });

        let resp: serde_json::Value = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("WxPusher send: {e}"))?
            .json()
            .await
            .map_err(|e| format!("WxPusher parse: {e}"))?;

        let success = resp["code"].as_i64() == Some(1000);
        let message = resp["msg"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        Ok(PushResult { success, message })
    }
}
