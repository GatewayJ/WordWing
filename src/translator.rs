// src/translator.rs
use reqwest;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Serialize)]
struct TranslationRequest {
    model: String,
    input: TranslationInput,
}

#[derive(Serialize)]
struct TranslationInput {
    prompt: String,
}

#[derive(Deserialize)]
struct TranslationResponse {
    output: TranslationOutput,
}

#[derive(Deserialize)]
struct TranslationOutput {
    text: String,
}

pub struct Translator {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl Translator {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url:
                "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation"
                    .to_string(),
        }
    }

    pub async fn translate(
        &self,
        text: &str,
        target_lang: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = format!("请将以下文本翻译成{}:\n\n{}", target_lang, text);

        let request_body = TranslationRequest {
            model: "qwen-turbo".to_string(),
            input: TranslationInput { prompt },
        };

        let response = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let result: TranslationResponse = response.json().await?;
        info!("Translation response received,{:?}", result.output.text);
        Ok(result.output.text)
    }
}
