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

    async fn push(&self, digest: &Digest, content: &str) -> Result<PushResult, String> {
        // WeCom markdown has limited support, build a compact version
        let mut md = render_brief_markdown(digest);

        // Extract stock and policy sections from full content
        if let Some(stock_part) = extract_section(content, "\u{1f4c8} \u{80a1}\u{5e02}\u{884c}\u{60c5}") {
            md.push_str(&format!("\n\n{}", stock_part));
        }
        if let Some(policy_part) = extract_section(content, "\u{1f4dc} \u{653f}\u{7b56}\u{6cd5}\u{89c4}\u{89e3}\u{8bfb}") {
            md.push_str(&format!("\n\n{}", policy_part));
        }

        let body = json!({
            "msgtype": "markdown",
            "markdown": { "content": md }
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

fn extract_section(content: &str, header: &str) -> Option<String> {
    let start = content.find(header)?;
    // Find the end: next "## " header or end of string
    let rest = &content[start..];
    let end = rest[header.len()..].find("\n## ").map(|i| start + header.len() + i);
    let section = match end {
        Some(e) => &content[start..e],
        None => &content[start..],
    };

    // Compact: only keep title lines, bold, and links
    let mut compact = String::new();
    for line in section.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Skip lines that are too long or have complex markdown
        if trimmed.len() > 200 {
            // Truncate long lines
            compact.push_str(&format!("{}\n", &trimmed[..trimmed.len().min(200)]));
            continue;
        }
        compact.push_str(trimmed);
        compact.push('\n');
    }

    if compact.is_empty() {
        None
    } else {
        Some(compact.trim_end().to_string())
    }
}
