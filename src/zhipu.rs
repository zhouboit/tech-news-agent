use crate::config::AppConfig;
use crate::models::{AiAnalysis, NewsItem};
use serde::Deserialize;
use tracing::{info, warn};

const API_URL: &str = "https://open.bigmodel.cn/api/paas/v4/chat/completions";
const BATCH_SIZE: usize = 15;

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

pub async fn analyze_items(items: &mut [NewsItem], config: &AppConfig) {
    let api_key = match &config.zhipu_api_key {
        Some(key) => key.clone(),
        None => {
            info!("ZHIPU_API_KEY not set, skipping AI analysis");
            return;
        }
    };

    if items.is_empty() {
        return;
    }

    let client = reqwest::Client::builder()
        .user_agent("TechNewsAgent/0.1")
        .build()
        .expect("build zhipu client");

    let total = items.len();
    let mut analyzed = 0usize;

    for chunk_start in (0..total).step_by(BATCH_SIZE) {
        let chunk_end = (chunk_start + BATCH_SIZE).min(total);
        let offset = chunk_start;

        let items_text = items[chunk_start..chunk_end]
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let source = item.source.display_name();
                let tags = item.tags.join(", ");
                let summary = item.summary.as_deref().unwrap_or("-");
                let author = item.author.as_deref().unwrap_or("-");
                format!(
                    "[{}] 标题: {}\n  来源: {}\n  作者: {}\n  标签: {}\n  简介: {}",
                    i + 1, item.title, source, author, tags, summary
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let system_prompt = "你是技术情报分析专家。对每条资讯返回JSON数组，每项包含：
- index: 序号（从1开始）
- brief_name: 中文简称（10字内）
- keywords: 3-5个中文关键词（字符串数组）
- summary_cn: 核心内容摘要（40-80字）
- impact: 影响分析（30-60字，对行业/技术/开发者的影响）
- action: 行动建议（20-40字，读者应关注或做什么）
- prediction: 趋势预测（20-40字，该技术方向未来展望）

严格返回JSON数组，不加任何额外文字或markdown标记。示例：
[{\"index\":1,\"brief_name\":\"...\",\"keywords\":[\"...\"],\"summary_cn\":\"...\",\"impact\":\"...\",\"action\":\"...\",\"prediction\":\"...\"}]";

        let user_prompt = format!("分析以下{}条技术资讯：\n\n{}", chunk_end - chunk_start, items_text);

        info!(
            "Zhipu AI: batch {}/{} (items {}-{}) with model {}",
            chunk_start / BATCH_SIZE + 1,
            (total + BATCH_SIZE - 1) / BATCH_SIZE,
            chunk_start + 1,
            chunk_end,
            config.zhipu_model
        );

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
                        warn!("Zhipu AI response parse error: {e}");
                        continue;
                    }
                };

                let content = match chat_resp.choices.first().and_then(|c| c.message.content.as_ref()) {
                    Some(c) => c.clone(),
                    None => {
                        warn!("Zhipu AI: empty response");
                        continue;
                    }
                };

                let content = extract_json_array(&content);
                let count = apply_analyses(&content, &mut items[offset..chunk_end]);
                analyzed += count;
            }
            Err(e) => {
                warn!("Zhipu AI request error: {e}");
            }
        }
    }

    info!("Zhipu AI: total analyzed {} / {} items", analyzed, total);
}

fn apply_analyses(json_str: &str, items: &mut [NewsItem]) -> usize {
    let mut count = 0usize;

    // Try to parse the full JSON array first
    if let Ok(analyses) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
        for analysis in &analyses {
            if try_apply_one(analysis, items) {
                count += 1;
            }
        }
        return count;
    }

    // Fallback: try to recover individual JSON objects from truncated array
    warn!("Zhipu AI: full JSON parse failed, attempting item-level recovery");
    let chars: Vec<char> = json_str.chars().collect();
    let mut depth = 0i32;
    let mut obj_start = None;

    for (i, &ch) in chars.iter().enumerate() {
        match ch {
            '{' if depth == 0 => {
                depth = 1;
                obj_start = Some(i);
            }
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = obj_start {
                        let obj_str: String = chars[start..=i].iter().collect();
                        if let Ok(analysis) = serde_json::from_str::<serde_json::Value>(&obj_str) {
                            if try_apply_one(&analysis, items) {
                                count += 1;
                            }
                        }
                    }
                    obj_start = None;
                }
            }
            _ => {}
        }
    }

    if count > 0 {
        info!("Zhipu AI: recovered {} items from partial JSON", count);
    }

    count
}

fn try_apply_one(analysis: &serde_json::Value, items: &mut [NewsItem]) -> bool {
    let index = analysis["index"].as_u64().unwrap_or(0) as usize;
    if index == 0 || index > items.len() {
        return false;
    }
    if let (Some(brief_name), Some(keywords), Some(summary_cn)) = (
        analysis["brief_name"].as_str(),
        analysis["keywords"].as_array(),
        analysis["summary_cn"].as_str(),
    ) {
        let keywords: Vec<String> = keywords
            .iter()
            .filter_map(|k| k.as_str().map(|s| s.to_string()))
            .collect();
        if !brief_name.is_empty() && !summary_cn.is_empty() {
            items[index - 1].ai_analysis = Some(AiAnalysis {
                brief_name: brief_name.to_string(),
                keywords,
                summary_cn: summary_cn.to_string(),
                impact: analysis["impact"].as_str().unwrap_or("").to_string(),
                action: analysis["action"].as_str().unwrap_or("").to_string(),
                prediction: analysis["prediction"].as_str().unwrap_or("").to_string(),
            });
            return true;
        }
    }
    false
}

fn extract_json_array(s: &str) -> String {
    let trimmed = s.trim();
    if let (Some(start), Some(end)) = (trimmed.find('['), trimmed.rfind(']')) {
        trimmed[start..=end].to_string()
    } else {
        trimmed.to_string()
    }
}
