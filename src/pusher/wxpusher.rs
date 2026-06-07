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

        let mut sections = Vec::new();
        sections.push(format!("\u{1f4f0} {}条资讯", digest.total_items));
        if content.contains("\u{1f4c8} \u{80a1}\u{5e02}\u{884c}\u{60c5}") {
            sections.push("\u{1f4c8}\u{80a1}\u{5e02}".to_string());
        }
        if content.contains("\u{1f4dc} \u{653f}\u{7b56}\u{6cd5}\u{89c4}\u{89e3}\u{8bfb}") {
            sections.push("\u{1f4dc}\u{653f}\u{7b56}".to_string());
        }
        if content.contains("\u{1f52c} \u{60c5}\u{62a5}\u{6d1e}\u{5bdf}") {
            sections.push("\u{1f52c}\u{60c5}\u{62a5}".to_string());
        }

        let body = json!({
            "appToken": self.token,
            "content": content,
            "summary": sections.join(" | "),
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
