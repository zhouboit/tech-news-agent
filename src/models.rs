use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SourceKind {
    HackerNews,
    GitHub,
    RustBlog,
    DevTo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Digest {
    pub generated_at: DateTime<Utc>,
    pub total_items: usize,
    pub categories: Vec<Category>,
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
