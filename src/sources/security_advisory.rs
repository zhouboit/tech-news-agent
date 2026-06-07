use crate::config::AppConfig;
use crate::models::{NewsItem, SourceKind};
use crate::sources::NewsSource;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::info;

#[derive(Deserialize)]
struct GhAdvisory {
    summary: Option<String>,
    html_url: Option<String>,
    severity: Option<String>,
    cve_id: Option<String>,
    published_at: Option<DateTime<Utc>>,
    vulnerability_package_name: Option<String>,
    ecosystem: Option<String>,
}

pub struct SecurityAdvisorySource {
    client: reqwest::Client,
}

impl SecurityAdvisorySource {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("TechNewsAgent/0.1")
            .build()
            .expect("build security advisory client");
        Self { client }
    }
}

#[async_trait]
impl NewsSource for SecurityAdvisorySource {
    fn name(&self) -> &str {
        "SecurityAdvisory"
    }

    async fn fetch(&self, config: &AppConfig) -> Result<Vec<NewsItem>, String> {
        let per_page = config.max_items_per_source;
        let url = format!(
            "https://api.github.com/advisories?per_page={}&sort=published&direction=desc",
            per_page
        );

        let advisories: Vec<GhAdvisory> = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .map_err(|e| format!("SecurityAdvisory fetch: {e}"))?
            .json()
            .await
            .map_err(|e| format!("SecurityAdvisory parse: {e}"))?;

        let items: Vec<NewsItem> = advisories
            .into_iter()
            .filter_map(|a| {
                let title = a.summary?;
                let url = a.html_url?;
                let mut tags = vec!["security".to_string(), "cve".to_string()];
                if let Some(ref cve) = a.cve_id { tags.push(cve.clone()); }
                if let Some(ref pkg) = a.vulnerability_package_name { tags.push(pkg.clone()); }
                if let Some(ref eco) = a.ecosystem { tags.push(eco.clone()); }
                if let Some(ref sev) = a.severity { tags.push(sev.clone()); }
                Some(NewsItem {
                    title,
                    url,
                    source: SourceKind::SecurityAdvisory,
                    score: severity_to_score(a.severity.as_deref()),
                    summary: a.vulnerability_package_name,
                    published_at: a.published_at,
                    tags,
                    author: None,
                    ai_analysis: None,
                })
            })
            .collect();

        info!("SecurityAdvisory: got {} items", items.len());
        Ok(items)
    }
}

fn severity_to_score(severity: Option<&str>) -> i64 {
    match severity {
        Some("CRITICAL") => 1000,
        Some("HIGH") => 500,
        Some("MEDIUM") => 200,
        Some("LOW") => 50,
        _ => 100,
    }
}
