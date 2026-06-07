use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAnalysis {
    pub brief_name: String,
    pub keywords: Vec<String>,
    pub summary_cn: String,
    pub impact: String,
    pub action: String,
    pub prediction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub title: String,
    pub url: String,
    pub source: SourceKind,
    pub score: i64,
    pub summary: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub ai_analysis: Option<AiAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SourceKind {
    HackerNews,
    GitHub,
    RustBlog,
    DevTo,
    Arxiv,
    SecurityAdvisory,
}

impl SourceKind {
    pub fn display_name(&self) -> &str {
        match self {
            SourceKind::HackerNews => "极客头条",
            SourceKind::GitHub => "GitHub",
            SourceKind::RustBlog => "Rust博客",
            SourceKind::DevTo => "Dev.to",
            SourceKind::Arxiv => "arXiv论文",
            SourceKind::SecurityAdvisory => "安全公告",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Digest {
    pub generated_at: DateTime<Utc>,
    pub total_items: usize,
    pub categories: Vec<Category>,
    #[serde(skip)]
    pub new_urls: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    pub emoji: String,
    pub items: Vec<NewsItem>,
}

#[derive(Debug)]
pub struct PushResult {
    pub success: bool,
    pub message: String,
}
