use crate::config::AppConfig;
use crate::models::{NewsItem, SourceKind};
use crate::sources::NewsSource;
use async_trait::async_trait;
use chrono::DateTime;
use quick_xml::events::Event;
use tracing::info;

pub struct ArxivSource {
    client: reqwest::Client,
}

impl ArxivSource {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("TechNewsAgent/0.1")
            .build()
            .expect("build arxiv client");
        Self { client }
    }
}

fn parse_arxiv_date(s: &str) -> Option<DateTime<chrono::Utc>> {
    let trimmed = s.trim();
    DateTime::parse_from_rfc3339(trimmed)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

struct Entry {
    title: String,
    id: String,
    summary: String,
    published: Option<DateTime<chrono::Utc>>,
    categories: Vec<String>,
    authors: Vec<String>,
}

fn parse_entries(xml: &str) -> Vec<Entry> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut entries = Vec::new();
    let mut current: Option<Entry> = None;
    let mut in_title = false;
    let mut in_id = false;
    let mut in_summary = false;
    let mut in_published = false;
    let mut in_category = false;
    let mut in_author_name = false;
    let mut depth = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.local_name();
                let name_str = std::str::from_utf8(name.as_ref()).unwrap_or("");
                match depth {
                    0 if name_str == "entry" => {
                        current = Some(Entry {
                            title: String::new(),
                            id: String::new(),
                            summary: String::new(),
                            published: None,
                            categories: Vec::new(),
                            authors: Vec::new(),
                        });
                    }
                    1 if name_str == "title" => in_title = true,
                    1 if name_str == "id" => in_id = true,
                    1 if name_str == "summary" => in_summary = true,
                    1 if name_str == "published" => in_published = true,
                    1 if name_str == "primary_category" => in_category = true,
                    2 if name_str == "name" => in_author_name = true,
                    _ => {}
                }
                depth += 1;
            }
            Ok(Event::Text(ref e)) => {
                if let Some(ref mut entry) = current {
                    let text = e.unescape().unwrap_or_default().to_string();
                    if in_title {
                        entry.title = text;
                    } else if in_id {
                        entry.id = text;
                    } else if in_summary {
                        entry.summary = text;
                    } else if in_published {
                        entry.published = parse_arxiv_date(&text);
                    } else if in_category && !text.is_empty() {
                        entry.categories.push(text);
                    } else if in_author_name && !text.is_empty() {
                        entry.authors.push(text);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                depth -= 1;
                let name = e.local_name();
                let name_str = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if in_title && name_str == "title" {
                    in_title = false;
                } else if in_id && name_str == "id" {
                    in_id = false;
                } else if in_summary && name_str == "summary" {
                    in_summary = false;
                } else if in_published && name_str == "published" {
                    in_published = false;
                } else if in_category && name_str == "primary_category" {
                    in_category = false;
                } else if in_author_name && name_str == "name" {
                    in_author_name = false;
                } else if depth == 0 && name_str == "entry" {
                    if let Some(entry) = current.take() {
                        entries.push(entry);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                tracing::warn!("arXiv XML parse error: {e}");
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    entries
}

#[async_trait]
impl NewsSource for ArxivSource {
    fn name(&self) -> &str {
        "arXiv"
    }

    async fn fetch(&self, config: &AppConfig) -> Result<Vec<NewsItem>, String> {
        let url = format!(
            "http://export.arxiv.org/api/query?search_query=cat:cs.AI+OR+cat:cs.LG&sortBy=submittedDate&sortOrder=descending&max_results={}",
            config.max_items_per_source
        );
        let xml: String = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("arXiv fetch: {e}"))?
            .text()
            .await
            .map_err(|e| format!("arXiv read body: {e}"))?;

        let entries = parse_entries(&xml);
        let items: Vec<NewsItem> = entries
            .into_iter()
            .map(|e| {
                let clean_title = e.title.split_whitespace().collect::<Vec<_>>().join(" ");
                let summary: String = e.summary.chars().take(200).collect();
                let mut tags = vec!["arxiv".to_string()];
                tags.extend(e.categories);
                NewsItem {
                    title: clean_title,
                    url: e.id,
                    source: SourceKind::Arxiv,
                    score: 0,
                    summary: Some(summary),
                    published_at: e.published,
                    tags,
                    author: e.authors.first().cloned(),
                    ai_analysis: None,
                }
            })
            .collect();

        info!("arXiv: got {} papers", items.len());
        Ok(items)
    }
}
