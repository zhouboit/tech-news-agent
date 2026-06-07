use crate::config::AppConfig;
use serde::Deserialize;
use tracing::{info, warn};

const API_URL: &str = "https://open.bigmodel.cn/api/paas/v4/chat/completions";

// Multiple channels: 0425=重要财经, 0520=社会政策, 0475=财经热点
const POLICY_CHANNELS: &[(&str, &str)] = &[
    ("90.BK0425", "重要财经"),
    ("90.BK0520", "社会政策"),
    ("90.BK0475", "财经热点"),
];

const CATEGORIES: &[(&str, &str)] = &[
    ("\u{793e}\u{4f1a}", "\u{1f468}"),
    ("\u{7ecf}\u{6d4e}", "\u{1f4b0}"),
    ("\u{5236}\u{9020}", "\u{1f3ed}"),
    ("\u{6280}\u{672f}", "\u{1f6e0}"),
];

#[derive(Debug)]
pub struct PolicyNews {
    pub title: String,
    pub url: String,
    pub source: String,
    pub category: String,
    pub emoji: String,
    pub summary: Option<String>,
    pub related_stocks: Vec<PolicyStock>,
}

#[derive(Debug)]
pub struct PolicyStock {
    pub code: String,
    pub name: String,
    pub reason: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: Option<String>,
}

pub async fn fetch_policy_news(client: &reqwest::Client) -> Vec<PolicyNews> {
    info!("Policy: fetching news...");

    let mut all_news = Vec::new();
    let mut seen_titles = std::collections::HashSet::new();

    for (code, label) in POLICY_CHANNELS {
        let url = format!(
            "https://np-listapi.eastmoney.com/comm/web/getListInfo?client=web&type=1&pageSize=5&pageindex=1&order=1&fields=Art_Title,Art_Url,Art_ShowTime,Art_SourceName&mTypeAndCode={}",
            code
        );

        let resp = client.get(&url).send().await;
        match resp {
            Ok(resp) => {
                let text = match resp.text().await {
                    Ok(t) => t,
                    Err(e) => {
                        warn!("Policy [{}]: read body error: {e}", label);
                        continue;
                    }
                };
                match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(val) => {
                        if let Some(arr) = val["data"]["list"].as_array() {
                            for item in arr.iter().take(5) {
                                let title = match item["Art_Title"].as_str() {
                                    Some(t) => t.to_string(),
                                    None => continue,
                                };
                                if !seen_titles.insert(title.clone()) {
                                    continue;
                                }
                                let url = item["Art_Url"].as_str().unwrap_or("").to_string();
                                let source = item["Art_SourceName"].as_str().unwrap_or(label).to_string();
                                all_news.push(PolicyNews {
                                    title,
                                    url,
                                    source,
                                    category: String::new(),
                                    emoji: String::new(),
                                    summary: None,
                                    related_stocks: Vec::new(),
                                });
                            }
                        }
                        info!("Policy [{}]: got items, total now {}", label, all_news.len());
                    }
                    Err(e) => {
                        warn!("Policy [{}]: JSON parse error: {e}", label);
                    }
                }
            }
            Err(e) => {
                warn!("Policy [{}]: fetch error: {e}", label);
            }
        }
    }

    info!("Policy: total {} unique items fetched", all_news.len());
    all_news
}

pub async fn analyze_policies_with_ai(
    policies: &mut [PolicyNews],
    stock_watch_list: &[String],
    config: &AppConfig,
) {
    let api_key = match &config.zhipu_api_key {
        Some(key) => key.clone(),
        None => {
            info!("Policy: ZHIPU_API_KEY not set, skipping AI analysis");
            return;
        }
    };

    if policies.is_empty() {
        return;
    }

    let client = reqwest::Client::builder()
        .user_agent("TechNewsAgent/0.1")
        .build()
        .expect("build policy ai client");

    let news_text: String = policies
        .iter()
        .enumerate()
        .map(|(i, p)| format!("[{}] {} (来源: {})", i + 1, p.title, p.source))
        .collect::<Vec<_>>()
        .join("\n");

    let watch_hint = if stock_watch_list.is_empty() {
        String::new()
    } else {
        format!("关注列表 [{}] 可优先推荐，但不仅限于关注列表。", stock_watch_list.join(", "))
    };

    let system_prompt = &format!("你是政策法规分析专家和A股投资顾问。分析以下新闻，为每条：
1. category: 领域分类，只能选其一：社会、经济、制造、技术
2. summary: 中文摘要（30-50字，提炼政策要点和影响）
3. stocks: 推荐2-4只受该政策影响最大的A股（含代码、名称、关联理由）{}

严格返回JSON数组，每项：{{\"index\":1,\"category\":\"...\",\"summary\":\"...\",\"stocks\":[{{\"code\":\"...\",\"name\":\"...\",\"reason\":\"...\"}}]}}
无相关股票则stocks为空数组。不加额外文字。", watch_hint);

    let user_prompt = format!("分析以下{}条政策新闻：\n{}", policies.len(), news_text);

    info!("Policy AI: analyzing {} policies", policies.len());

    let body = serde_json::json!({
        "model": config.zhipu_model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ],
        "temperature": 0.3,
    });

    let resp = client
        .post(API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await;

    match resp {
        Ok(resp) => {
            let chat_resp: ChatResponse = match resp.json().await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Policy AI response parse error: {e}");
                    return;
                }
            };

            let content = match chat_resp.choices.first().and_then(|c| c.message.content.as_ref()) {
                Some(c) => c.clone(),
                None => {
                    warn!("Policy AI: empty response");
                    return;
                }
            };

            info!("Policy AI raw response (first 500 chars): {}", &content[..content.len().min(500)]);

            let json_str = extract_json(&content);
            info!("Policy AI extracted JSON (first 500 chars): {}", &json_str[..json_str.len().min(500)]);
            match serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
                Ok(analyses) => {
                    let mut count = 0usize;
                    for a in &analyses {
                        let index = a["index"].as_u64().unwrap_or(0) as usize;
                        if index == 0 || index > policies.len() {
                            continue;
                        }
                        let cat = a["category"].as_str().unwrap_or("").to_string();
                        let summary = a["summary"].as_str().unwrap_or("").to_string();
                        let stocks: Vec<PolicyStock> = a["stocks"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|s| {
                                        let code = s["code"].as_str()?.to_string();
                                        let name = s["name"].as_str()?.to_string();
                                        let reason = s["reason"].as_str()?.to_string();
                                        Some(PolicyStock { code, name, reason })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();

                        let emoji = CATEGORIES
                            .iter()
                            .find(|(c, _)| *c == cat)
                            .map(|(_, e)| e.to_string())
                            .unwrap_or_else(|| "\u{1f4cb}".to_string());

                        if !summary.is_empty() {
                            policies[index - 1].category = cat;
                            policies[index - 1].emoji = emoji;
                            policies[index - 1].summary = Some(summary);
                            policies[index - 1].related_stocks = stocks;
                            count += 1;
                        }
                    }
                    info!("Policy AI: analyzed {} / {} policies", count, policies.len());
                }
                Err(e) => {
                    warn!("Policy AI JSON parse error: {e}");
                }
            }
        }
        Err(e) => {
            warn!("Policy AI request error: {e}");
        }
    }
}

pub fn render_policy_section(policies: &[PolicyNews]) -> String {
    if policies.is_empty() {
        return String::new();
    }

    // Separate: analyzed items (have category+summary) vs unanalyzed items
    let analyzed: Vec<&PolicyNews> = policies.iter().filter(|p| p.summary.is_some()).collect();
    let unanalyzed: Vec<&PolicyNews> = policies.iter().filter(|p| p.summary.is_none()).collect();

    if analyzed.is_empty() && unanalyzed.is_empty() {
        return String::new();
    }

    let mut md = String::from("## \u{1f4dc} \u{653f}\u{7b56}\u{6cd5}\u{89c4}\u{89e3}\u{8bfb}\n\n");

    // Analyzed items: group by category
    if !analyzed.is_empty() {
        let mut groups: Vec<(String, String, Vec<&PolicyNews>)> = Vec::new();
        for p in &analyzed {
            let cat = p.category.clone();
            let emoji = p.emoji.clone();
            if let Some(g) = groups.iter_mut().find(|(c, _, _)| *c == cat) {
                g.2.push(p);
            } else {
                groups.push((cat, emoji, vec![p]));
            }
        }
        groups.sort_by(|a, b| b.2.len().cmp(&a.2.len()));

        for (cat, emoji, items) in &groups {
            if cat.is_empty() {
                continue;
            }
            md.push_str(&format!("### {} {}\n\n", emoji, cat));
            for p in items {
                md.push_str(&format!("- **{}** [\u{539f}\u{6587}]({})\n", p.title, p.url));
                if let Some(ref summary) = p.summary {
                    md.push_str(&format!("  \u{1f4dd} {}\n", summary));
                }
                for stock in &p.related_stocks {
                    md.push_str(&format!(
                        "  \u{1f4c1} [{}]({}) \u{2014} {}\n",
                        stock.name, stock.code, stock.reason
                    ));
                }
            }
            md.push('\n');
        }
    }

    // Unanalyzed items: show as plain list (AI analysis failed or skipped)
    if !unanalyzed.is_empty() {
        md.push_str(&format!("### \u{1f4cb} \u{653f}\u{7b56}\u{8d44}\u{8baf}\n\n"));
        for p in unanalyzed {
            md.push_str(&format!("- **{}** [\u{539f}\u{6587}]({})\n", p.title, p.url));
        }
        md.push('\n');
    }

    md
}

fn extract_json(s: &str) -> String {
    let trimmed = s.trim();
    if let (Some(start), Some(end)) = (trimmed.find('['), trimmed.rfind(']')) {
        trimmed[start..=end].to_string()
    } else {
        trimmed.to_string()
    }
}
