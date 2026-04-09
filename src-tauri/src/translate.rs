use reqwest::Client;
use serde_json::Value;

/// 是否包含常见汉字（CJK 统一表意文字），用于「中译英」模式校验。
pub fn is_mostly_chinese(text: &str) -> bool {
    text.chars()
        .any(|c| (c as u32) >= 0x4e00 && (c as u32) <= 0x9fff)
}

/// 中译英固定目标语言标签（与 DashScope 提示语一致）。
pub const TARGET_ENGLISH: &str = "English";

pub fn target_language_label(text: &str) -> &'static str {
    if is_mostly_chinese(text) {
        TARGET_ENGLISH
    } else {
        "中文"
    }
}

pub async fn translate_dashscope(
    api_key: &str,
    text: &str,
    target_lang: &str,
) -> Result<String, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let prompt = format!("请将以下文本翻译成{}:\n\n{}", target_lang, text);

    let body = serde_json::json!({
        "model": "qwen-turbo",
        "input": { "prompt": prompt }
    });

    let res = client
        .post("https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = res.status();
    let val: Value = res.json().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(val
            .pointer("/message")
            .and_then(|v| v.as_str())
            .unwrap_or("翻译请求失败")
            .to_string());
    }

    extract_translation_text(&val)
}

fn extract_translation_text(val: &Value) -> Result<String, String> {
    if let Some(t) = val.pointer("/output/text").and_then(|v| v.as_str()) {
        let t = t.trim();
        if !t.is_empty() {
            return Ok(t.to_string());
        }
    }
    if let Some(choices) = val.pointer("/output/choices").and_then(|v| v.as_array()) {
        if let Some(first) = choices.first() {
            if let Some(t) = first
                .pointer("/message/content")
                .and_then(|v| v.as_str())
                .or_else(|| first.pointer("/text").and_then(|v| v.as_str()))
            {
                let t = t.trim();
                if !t.is_empty() {
                    return Ok(t.to_string());
                }
            }
        }
    }
    Err(format!("无法解析译文: {}", val))
}

/// 模板见 `prompts/weekly_article.txt`，占位符 `{{PHRASES}}` 由调用方替换为编号短语列表。
const WEEKLY_ARTICLE_PROMPT_TEMPLATE: &str = include_str!("../prompts/weekly_article.txt");

/// 是否与翻译浮层共用：环境变量 `DASHSCOPE_API_KEY` 非空。
pub fn dashscope_api_key_configured() -> bool {
    std::env::var("DASHSCOPE_API_KEY")
        .ok()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

/// 周短文：仅返回模型输出字符串（JSON），由 weekly_article 解析。
pub async fn weekly_article_completion(
    api_key: &str,
    phrase_block: &str,
) -> Result<String, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| e.to_string())?;

    let prompt = WEEKLY_ARTICLE_PROMPT_TEMPLATE.replace("{{PHRASES}}", phrase_block);

    let body = serde_json::json!({
        "model": "qwen-turbo",
        "input": { "prompt": prompt }
    });

    let res = client
        .post("https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = res.status();
    let val: Value = res.json().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(val
            .pointer("/message")
            .and_then(|v| v.as_str())
            .unwrap_or("周短文生成请求失败")
            .to_string());
    }

    extract_translation_text(&val)
}
