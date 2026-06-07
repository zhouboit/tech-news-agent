use crate::config::AppConfig;
use chrono::Local;
use serde::Deserialize;
use tracing::{info, warn};

const API_URL: &str = "https://open.bigmodel.cn/api/paas/v4/chat/completions";

pub struct StockQuote {
    pub code: String,
    pub name: String,
    pub current: f64,
    pub open: f64,
    #[allow(dead_code)]
    pub yesterday_close: f64,
    pub high: f64,
    pub low: f64,
    pub change_pct: f64,
    pub volume: f64,
    pub amount: f64,
    pub analysis: Option<String>,
}

pub async fn fetch_stocks(config: &AppConfig) -> Vec<StockQuote> {
    let codes = &config.stock_watch_list;
    if codes.is_empty() {
        return Vec::new();
    }

    let client = reqwest::Client::builder()
        .user_agent("TechNewsAgent/0.1")
        .build()
        .expect("build stock client");

    let query_codes: Vec<String> = codes
        .iter()
        .map(|c| {
            if c.starts_with('6') || c.starts_with('9') {
                format!("sh{}", c)
            } else {
                format!("sz{}", c)
            }
        })
        .collect();

    let url = format!("http://hq.sinajs.cn/list={}", query_codes.join(","));

    info!("Stock: fetching {}", codes.join(", "));

    let resp = client
        .get(&url)
        .header("Referer", "https://finance.sina.com.cn")
        .send()
        .await;

    match resp {
        Ok(resp) => {
            let text = match resp.text().await {
                Ok(t) => t,
                Err(e) => {
                    warn!("Stock: read body error: {e}");
                    return Vec::new();
                }
            };

            let mut quotes = Vec::new();
            for line in text.lines() {
                if let Some(quote) = parse_sina_line(line) {
                    quotes.push(quote);
                }
            }

            info!("Stock: got {} quotes", quotes.len());
            quotes
        }
        Err(e) => {
            warn!("Stock: fetch error: {e}");
            Vec::new()
        }
    }
}

fn parse_sina_line(line: &str) -> Option<StockQuote> {
    let eq_pos = line.find('"')?;
    let data = &line[eq_pos + 1..];
    let end = data.rfind('"')?;
    let data = &data[..end];

    let fields: Vec<&str> = data.split(',').collect();
    if fields.len() < 32 {
        return None;
    }

    let name = fields[0].to_string();
    let open: f64 = fields[1].parse().ok()?;
    let yesterday_close: f64 = fields[2].parse().ok()?;
    let current: f64 = fields[3].parse().ok()?;
    let high: f64 = fields[4].parse().ok()?;
    let low: f64 = fields[5].parse().ok()?;
    let volume: f64 = fields[8].parse().ok().unwrap_or(0.0);
    let amount: f64 = fields[9].parse().ok().unwrap_or(0.0);

    let change_pct = if yesterday_close > 0.0 {
        (current - yesterday_close) / yesterday_close * 100.0
    } else {
        0.0
    };

    let code = line
        .find("hq_str_")
        .and_then(|pos| {
            let after = &line[pos + 8..];
            let prefix = if after.starts_with("sh") || after.starts_with("sz") { 2 } else { 0 };
            after.get(prefix..).and_then(|s| s.split('"').next().map(|c| c.to_string()))
        })
        .unwrap_or_default();

    Some(StockQuote {
        code,
        name,
        current,
        open,
        yesterday_close,
        high,
        low,
        change_pct,
        volume,
        amount,
        analysis: None,
    })
}

// --- Zhipu AI stock analysis ---

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

pub async fn analyze_stocks_with_ai(quotes: &mut [StockQuote], config: &AppConfig) {
    let api_key = match &config.zhipu_api_key {
        Some(key) => key.clone(),
        None => return,
    };

    if quotes.is_empty() {
        return;
    }

    let client = reqwest::Client::builder()
        .user_agent("TechNewsAgent/0.1")
        .build()
        .expect("build stock ai client");

    let stock_data: String = quotes
        .iter()
        .map(|q| {
            format!(
                "[{}] {} 现价:{:.2} 涨跌幅:{:.2}% 开盘:{:.2} 最高:{:.2} 最低:{:.2} 成交额:{:.0}万",
                q.code, q.name, q.current, q.change_pct,
                q.open, q.high, q.low, q.amount / 10000.0
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let system_prompt = "你是资深A股分析师。根据以下股票实时行情数据，对每只股票给出简短分析，包括：
1. 当日走势点评（20-30字）
2. 关注要点或风险提示（20-30字）

严格返回JSON数组，每项包含 index（序号从1开始）、comment（点评）、focus（关注要点）。不加额外文字。";

    let user_prompt = format!("分析以下股票行情：\n{}", stock_data);

    info!("Stock AI: analyzing {} stocks", quotes.len());

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
                    warn!("Stock AI response parse error: {e}");
                    return;
                }
            };

            let content = match chat_resp.choices.first().and_then(|c| c.message.content.as_ref()) {
                Some(c) => c.clone(),
                None => {
                    warn!("Stock AI: empty response");
                    return;
                }
            };

            let content = extract_json(&content);
            match serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                Ok(analyses) => {
                    let mut count = 0usize;
                    for a in &analyses {
                        let index = a["index"].as_u64().unwrap_or(0) as usize;
                        if index == 0 || index > quotes.len() {
                            continue;
                        }
                        let comment = a["comment"].as_str().unwrap_or("");
                        let focus = a["focus"].as_str().unwrap_or("");
                        if !comment.is_empty() {
                            let text = if focus.is_empty() {
                                comment.to_string()
                            } else {
                                format!("{}\u{ff1a}{}", comment, focus)
                            };
                            quotes[index - 1].analysis = Some(text);
                            count += 1;
                        }
                    }
                    info!("Stock AI: analyzed {} / {} stocks", count, quotes.len());
                }
                Err(e) => {
                    warn!("Stock AI JSON parse error: {e}");
                }
            }
        }
        Err(e) => {
            warn!("Stock AI request error: {e}");
        }
    }
}

fn extract_json(s: &str) -> String {
    let trimmed = s.trim();
    if let (Some(start), Some(end)) = (trimmed.find('['), trimmed.rfind(']')) {
        trimmed[start..=end].to_string()
    } else {
        trimmed.to_string()
    }
}

// --- Hot stock news ---

#[derive(Debug)]
pub struct StockNews {
    pub title: String,
    pub url: String,
}

pub async fn fetch_stock_news(client: &reqwest::Client, max_items: usize) -> Vec<StockNews> {
    // Use East Money finance API for A-share hot news
    let url = format!(
        "https://np-listapi.eastmoney.com/comm/web/getListInfo?client=web&type=1&pageSize={}&pageindex=1&order=1&fields=title,url,showTime",
        max_items
    );

    match client.get(&url).send().await {
        Ok(resp) => {
            let text = match resp.text().await {
                Ok(t) => t,
                Err(_) => return Vec::new(),
            };

            // East Money returns JSONP, extract JSON
            let json_str = extract_json(&text);
            match serde_json::from_str::<serde_json::Value>(&json_str) {
                Ok(val) => {
                    val["data"]["list"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .take(max_items)
                                .filter_map(|item| {
                                    let title = item["title"].as_str()?.to_string();
                                    let url = item["url"].as_str()?.to_string();
                                    Some(StockNews { title, url })
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                }
                Err(_) => Vec::new(),
            }
        }
        Err(_) => Vec::new(),
    }
}

// --- Render ---

pub fn render_stock_section(quotes: &[StockQuote], news: &[StockNews]) -> String {
    if quotes.is_empty() && news.is_empty() {
        return String::new();
    }

    let now = Local::now().format("%m-%d %H:%M");
    let mut md = format!(
        "## \u{1f4c8} \u{80a1}\u{5e02}\u{884c}\u{60c5}\n> {}\n\n",
        now
    );

    // Stock quotes with AI analysis
    for q in quotes {
        let arrow = if q.change_pct > 0.01 { "\u{1f4c8}" }
            else if q.change_pct < -0.01 { "\u{1f4c9}" }
            else { "\u{2192}" };
        let sign = if q.change_pct >= 0.0 { "+" } else { "" };
        md.push_str(&format!(
            "**{}** ({})\n{} **{:.2}** ({}{:.2}%) | \u{5f00}: {:.2} \u{9ad8}: {:.2} \u{4f4e}: {:.2} | \u{91cf}: {:.0}\u{624b} \u{989d}: {:.0}\u{4e07}\n",
            q.name, q.code, arrow, q.current, sign, q.change_pct,
            q.open, q.high, q.low,
            q.volume / 10000.0, q.amount / 10000.0,
        ));
        if let Some(ref analysis) = q.analysis {
            md.push_str(&format!("\u{1f50d} {}\n", analysis));
        }
        md.push('\n');
    }

    // Hot stock news
    if !news.is_empty() {
        md.push_str(&format!(
            "### \u{1f525} \u{80a1}\u{5e02}\u{70ed}\u{70b9}\n\n"
        ));
        for item in news {
            md.push_str(&format!("- {} [\u{539f}\u{6587}]({})\n", item.title, item.url));
        }
        md.push('\n');
    }

    md
}
