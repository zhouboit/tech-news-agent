use crate::models::{Category, Digest, NewsItem};
use chrono::Utc;

const CLASSIFY_RULES: &[(&str, &[&str])] = &[
    ("AI/ML", &["ai", "ml", "llm", "gpt", "machine learning", "deep learning", "transformer", "claude", "openai"]),
    ("Rust", &["rust", "cargo", "tokio", "wasm", "webassembly"]),
    ("Web", &["javascript", "typescript", "react", "vue", "nextjs", "css", "frontend", "node"]),
    ("\u{540e}\u{7aef}", &["database", "postgres", "redis", "kafka", "grpc", "microservice", "backend", "api"]),
    ("DevOps", &["kubernetes", "docker", "k8s", "terraform", "ci/cd", "devops", "cloud"]),
    ("\u{5b89}\u{5168}", &["security", "vulnerability", "cve", "exploit", "encryption"]),
    ("\u{5f00}\u{6e90}", &["open source", "oss", "github"]),
];

const EMOJI_MAP: &[(&str, &str)] = &[
    ("AI/ML", "\u{1f916}"),
    ("Rust", "\u{1f980}"),
    ("Web", "\u{1f310}"),
    ("\u{540e}\u{7aef}", "\u{1f5c4}\u{fe0f}"),
    ("DevOps", "\u{2601}\u{fe0f}"),
    ("\u{5b89}\u{5168}", "\u{1f512}"),
    ("\u{5f00}\u{6e90}", "\u{1f4e6}"),
    ("\u{5176}\u{4ed6}\u{6280}\u{672f}", "\u{1f4a1}"),
];

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

pub fn generate_digest(items: Vec<NewsItem>) -> Digest {
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
    }
}

pub fn render_markdown(digest: &Digest) -> String {
    let mut md = String::new();
    md.push_str(&format!(
        "# \u{1f4f0} Tech News Digest\n\n"
    ));
    md.push_str(&format!(
        "> \u{751f}\u{6210}\u{4e8e}: {} | \u{5171} {} \u{6761}\u{8d44}\u{8baf}\n\n",
        digest.generated_at.format("%Y-%m-%d %H:%M UTC"),
        digest.total_items
    ));

    for cat in &digest.categories {
        md.push_str(&format!("## {} {}\n\n", cat.emoji, cat.name));
        for (i, item) in cat.items.iter().enumerate() {
            md.push_str(&format!(
                "{}. [{}]({})",
                i + 1,
                item.title,
                item.url
            ));
            if let Some(ref summary) = item.summary {
                let s: String = summary.chars().take(100).collect();
                md.push_str(&format!(" - {}", s));
            }
            md.push_str("\n");
            if !item.tags.is_empty() {
                md.push_str(&format!(
                    "   \u{1f3f7}\u{fe0f} {}\n",
                    item.tags
                        .iter()
                        .map(|t| format!("`{}`", t))
                        .collect::<Vec<_>>()
                        .join(" ")
                ));
            }
        }
        md.push('\n');
    }

    md
}

pub fn render_brief_markdown(digest: &Digest) -> String {
    let mut md = String::new();
    md.push_str(&format!(
        "## \u{1f4f0} Tech News ({})\n",
        digest.generated_at.format("%m-%d %H:%M")
    ));

    for cat in &digest.categories {
        md.push_str(&format!("> {} **{}**\n", cat.emoji, cat.name));
        let top_items = cat.items.iter().take(5);
        for item in top_items {
            md.push_str(&format!("- [{}]({})\n", item.title, item.url));
        }
        md.push('\n');
    }

    md
}
