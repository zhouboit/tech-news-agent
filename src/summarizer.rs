use crate::models::{Category, Digest, NewsItem};
use chrono::{Local, Utc};
use std::collections::HashSet;

const CLASSIFY_RULES: &[(&str, &[&str])] = &[
    ("AI/ML", &["ai", "ml", "llm", "gpt", "machine learning", "deep learning", "transformer", "claude", "openai"]),
    ("Rust", &["rust", "cargo", "tokio", "wasm", "webassembly"]),
    ("Web", &["javascript", "typescript", "react", "vue", "nextjs", "css", "frontend", "node"]),
    ("\u{540e}\u{7aef}", &["database", "postgres", "redis", "kafka", "grpc", "microservice", "backend", "api"]),
    ("DevOps", &["kubernetes", "docker", "k8s", "terraform", "ci/cd", "devops", "cloud"]),
    ("\u{5b89}\u{5168}", &["security", "vulnerability", "cve", "exploit", "encryption"]),
    ("\u{5f00}\u{6e90}", &["open source", "oss", "github"]),
    ("\u{8bba}\u{6587}", &["paper", "arxiv", "research", "study", "neural", "benchmark", "dataset", "pretrain", "fine-tun", "attention mechanism"]),
    ("\u{4e13}\u{5229}", &["patent", "intellectual property", "trademark", "filing", "\u{4e13}\u{5229}"]),
];

const EMOJI_MAP: &[(&str, &str)] = &[
    ("AI/ML", "\u{1f916}"),
    ("Rust", "\u{1f980}"),
    ("Web", "\u{1f310}"),
    ("\u{540e}\u{7aef}", "\u{1f5c4}\u{fe0f}"),
    ("DevOps", "\u{2601}\u{fe0f}"),
    ("\u{5b89}\u{5168}", "\u{1f512}"),
    ("\u{5f00}\u{6e90}", "\u{1f4e6}"),
    ("\u{8bba}\u{6587}", "\u{1f4c4}"),
    ("\u{4e13}\u{5229}", "\u{1f4dc}"),
    ("\u{5176}\u{4ed6}\u{6280}\u{672f}", "\u{1f4a1}"),
];

const BREAKTHROUGH_KEYWORDS: &[&str] = &[
    "breakthrough", "novel", "state-of-the-art", "sota", "first", "record",
    "outperform", "surpass", "groundbreaking", "milestone", "leap",
];

const INSIGHT_HEADER: &str = "\u{1f52c} \u{60c5}\u{62a5}\u{6d1e}\u{5bdf}";
const BREAKTHROUGH_HEADER: &str = "\u{1f4a5} \u{6280}\u{672f}\u{7a81}\u{7834}\u{52a8}\u{5411}";
const CROSS_DOMAIN_HEADER: &str = "\u{1f517} \u{8de8}\u{754c}\u{5173}\u{8054}";

fn classify_by_keywords(title: &str, tags: &[String]) -> Vec<String> {
    let lower_title = title.to_lowercase();
    let lower_tags: Vec<String> = tags.iter().map(|t| t.to_lowercase()).collect();

    let mut matched = Vec::new();
    for &(category, keywords) in CLASSIFY_RULES {
        for &kw in keywords {
            if lower_title.contains(kw) || lower_tags.iter().any(|t| t.contains(kw)) {
                matched.push(category.to_string());
                break;
            }
        }
    }

    if matched.is_empty() {
        matched.push("\u{5176}\u{4ed6}\u{6280}\u{672f}".to_string());
    }
    matched
}

fn emoji_for(name: &str) -> &str {
    EMOJI_MAP
        .iter()
        .find(|(cat, _)| *cat == name)
        .map(|(_, e)| *e)
        .unwrap_or("\u{1f4a1}")
}

pub fn generate_digest(items: Vec<NewsItem>, new_urls: HashSet<String>) -> Digest {
    let mut category_map: std::collections::BTreeMap<String, Vec<NewsItem>> =
        std::collections::BTreeMap::new();

    for item in items {
        let categories = classify_by_keywords(&item.title, &item.tags);
        for cat in categories {
            category_map
                .entry(cat)
                .or_default()
                .push(item.clone());
        }
    }

    let mut categories: Vec<Category> = category_map
        .into_iter()
        .map(|(name, mut items)| {
            items.sort_by(|a, b| b.score.cmp(&a.score));
            let emoji = emoji_for(&name).to_string();
            Category { name, emoji, items }
        })
        .collect();

    categories.sort_by(|a, b| b.items.len().cmp(&a.items.len()));

    let total = categories.iter().map(|c| c.items.len()).sum();

    Digest {
        generated_at: Utc::now(),
        total_items: total,
        categories,
        new_urls,
    }
}

pub fn render_markdown(digest: &Digest) -> String {
    let mut md = String::new();
    md.push_str(&format!(
        "# \u{1f4f0} \u{6280}\u{672f}\u{8d44}\u{8baf}\u{65e5}\u{62a5}\n> {} | {}\u{6761}\u{8d44}\u{8baf}\n\n",
        digest.generated_at.with_timezone(&Local).format("%Y-%m-%d %H:%M"),
        digest.total_items
    ));

    for cat in &digest.categories {
        md.push_str(&format!("## {} {}\n\n", cat.emoji, cat.name));
        for (i, item) in cat.items.iter().enumerate() {
            let date_str = item
                .published_at
                .map(|dt| dt.with_timezone(&Local).format("%m-%d").to_string())
                .unwrap_or_else(|| "-".to_string());
            let source_name = item.source.display_name();
            md.push_str(&format!("**{}. {}**\n", i + 1, item.title));

            if let Some(ref ai) = item.ai_analysis {
                md.push_str(&format!(
                    "\u{1f3f7}\u{fe0f} {}\n",
                    ai.keywords.iter().map(|k| format!("`{}`", k)).collect::<Vec<_>>().join(" ")
                ));
                md.push_str(&format!("\u{1f4dd} {}\n", ai.summary_cn));
                if !ai.impact.is_empty() {
                    md.push_str(&format!("\u{1f4ca} {}\n", ai.impact));
                }
                if !ai.action.is_empty() {
                    md.push_str(&format!("\u{1f4cb} {}\n", ai.action));
                }
                if !ai.prediction.is_empty() {
                    md.push_str(&format!("\u{1f52d}\u{fe0f} {}\n", ai.prediction));
                }
            } else if let Some(ref summary) = item.summary {
                let s: String = summary.chars().take(120).collect();
                md.push_str(&format!("\u{1f4dd} {}\n", s));
            }

            let new_tag = if digest.new_urls.contains(&item.url) { " [NEW]" } else { "" };
            md.push_str(&format!(
                "\u{1f4c1} {} | {}{}、[\u{539f}\u{6587}]({})\n",
                date_str, source_name, new_tag, item.url
            ));
            md.push('\n');
        }
    }

    // Intelligence insights (deduplicate by url)
    let mut cross_domain = Vec::new();
    let mut breakthroughs = Vec::new();
    let mut seen_cross: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut seen_break: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for cat in &digest.categories {
        for item in &cat.items {
            if !seen_break.insert(&item.url) {
                continue;
            }
            let cats = classify_by_keywords(&item.title, &item.tags);
            if cats.len() >= 2 && seen_cross.insert(&item.url) {
                cross_domain.push((item, cats));
            } else if cats.len() < 2 {
                seen_cross.insert(&item.url);
            }
            let lower = item.title.to_lowercase();
            if BREAKTHROUGH_KEYWORDS.iter().any(|kw| lower.contains(kw)) {
                breakthroughs.push(item);
            }
        }
    }

    if !cross_domain.is_empty() || !breakthroughs.is_empty() {
        md.push_str("---\n\n");
        md.push_str(&format!("## {}\n\n", INSIGHT_HEADER));

        if !breakthroughs.is_empty() {
            md.push_str(&format!("### {}\n\n", BREAKTHROUGH_HEADER));
            for item in breakthroughs.iter().take(10) {
                let name = item.ai_analysis.as_ref()
                    .map(|a| a.brief_name.as_str())
                    .unwrap_or(&item.title);
                md.push_str(&format!("- {} [原文]({})\n", name, item.url));
            }
            md.push('\n');
        }

        if !cross_domain.is_empty() {
            md.push_str(&format!("### {}\n\n", CROSS_DOMAIN_HEADER));
            for (item, cats) in cross_domain.iter().take(10) {
                let name = item.ai_analysis.as_ref()
                    .map(|a| a.brief_name.as_str())
                    .unwrap_or(&item.title);
                md.push_str(&format!(
                    "- {} [原文]({}) ({})\n",
                    name, item.url, cats.join(" \u{2194} ")
                ));
            }
            md.push('\n');
        }
    }

    md
}

pub fn render_brief_markdown(digest: &Digest) -> String {
    let mut md = String::new();
    md.push_str(&format!(
        "## \u{1f4f0} \u{6280}\u{672f}\u{8d44}\u{8baf} ({})\n",
        digest.generated_at.with_timezone(&Local).format("%m-%d %H:%M")
    ));

    for cat in &digest.categories {
        md.push_str(&format!("> {} **{}**\n", cat.emoji, cat.name));
        for item in cat.items.iter().take(5) {
            let name = item.ai_analysis.as_ref()
                .map(|a| a.brief_name.as_str())
                .unwrap_or(&item.title);
            let new_tag = if digest.new_urls.contains(&item.url) { " [NEW]" } else { "" };
            md.push_str(&format!("- {}{} [原文]({})\n", name, new_tag, item.url));
        }
        md.push('\n');
    }

    md
}
