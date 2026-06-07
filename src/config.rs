use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub serverchan_key: Option<String>,
    pub wxpusher_token: Option<String>,
    pub wxpusher_uids: Vec<String>,
    pub wecom_webhook: Option<String>,
    pub fetch_interval_minutes: u64,
    pub max_items_per_source: usize,
    pub min_score: i64,
    pub github_langs: Vec<String>,
    pub zhipu_api_key: Option<String>,
    pub zhipu_model: String,
    pub stock_watch_list: Vec<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            serverchan_key: env::var("SERVERCHAN_KEY").ok().filter(|s| !s.is_empty()),
            wxpusher_token: env::var("WXPUSHER_TOKEN").ok().filter(|s| !s.is_empty()),
            wxpusher_uids: env::var("WXPUSHER_UIDS")
                .ok()
                .filter(|s| !s.is_empty())
                .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
                .unwrap_or_default(),
            wecom_webhook: env::var("WECOM_WEBHOOK").ok().filter(|s| !s.is_empty()),
            fetch_interval_minutes: env::var("FETCH_INTERVAL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
            max_items_per_source: env::var("MAX_ITEMS_PER_SOURCE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            min_score: env::var("MIN_SCORE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            github_langs: env::var("GITHUB_LANG")
                .ok()
                .filter(|s| !s.is_empty())
                .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
                .unwrap_or_else(|| {
                    vec![
                        "rust".to_string(),
                        "golang".to_string(),
                        "java".to_string(),
                        "python".to_string(),
                        "typescript".to_string(),
                    ]
                }),
            zhipu_api_key: env::var("ZHIPU_API_KEY").ok().filter(|s| !s.is_empty()),
            zhipu_model: env::var("ZHIPU_MODEL")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "glm-4".to_string()),
            stock_watch_list: env::var("STOCK_WATCH_LIST")
                .ok()
                .filter(|s| !s.is_empty())
                .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
                .unwrap_or_else(|| vec!["688326".to_string(), "600967".to_string()]),
        }
    }

    pub fn has_push_channel(&self) -> bool {
        self.serverchan_key.is_some()
            || (self.wxpusher_token.is_some() && !self.wxpusher_uids.is_empty())
            || self.wecom_webhook.is_some()
    }
}
