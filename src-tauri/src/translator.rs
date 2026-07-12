use std::time::Duration;

use serde_json::Value;

use crate::{
    error::{AppError, AppResult},
    models::{AppConfig, TranslateResponse, TranslationEngine},
};

pub async fn translate_text_with_config(
    config: &AppConfig,
    text: String,
    target_lang: String,
) -> AppResult<TranslateResponse> {
    let source = text.trim();
    if source.is_empty() {
        return Err(AppError::InvalidInput("翻译文本不能为空".to_string()));
    }

    match config.translation_engine {
        TranslationEngine::Ai => translate_ai(config, source, &target_lang).await,
        TranslationEngine::Google => translate_google(config, source, &target_lang).await,
    }
}

async fn translate_ai(
    config: &AppConfig,
    text: &str,
    target_lang: &str,
) -> AppResult<TranslateResponse> {
    let api_url = config.translation_api_url.trim();
    let api_key = config.translation_api_key.trim();
    let model = config.translation_model.trim();

    if api_url.is_empty() || api_key.is_empty() {
        return Err(AppError::InvalidInput(
            "AI 翻译未配置，请在设置中填写 API 地址和 Key".to_string(),
        ));
    }

    let full_url = if api_url.contains("/chat/completions") || api_url.contains("/completions") {
        api_url.to_string()
    } else {
        format!("{}/v1/chat/completions", api_url.trim_end_matches('/'))
    };
    let model = if model.is_empty() { "gpt-4o-mini" } else { model };
    let prompt = format!(
        "Translate the following text to {target_lang}. Only output the translated text, nothing else.\n\nText: {text}",
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| AppError::InvalidInput(format!("创建 HTTP 客户端失败：{error}")))?;

    let response = client
        .post(&full_url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a professional translator. Only output the translated text."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.3
        }))
        .send()
        .await
        .map_err(|error| AppError::InvalidInput(format!("AI 翻译请求失败：{error}")))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| AppError::InvalidInput(format!("读取 AI 响应失败：{error}")))?;

    if !status.is_success() {
        return Err(AppError::InvalidInput(format!(
            "AI 翻译 HTTP {}：{}",
            status.as_u16(),
            truncate_for_error(&body)
        )));
    }

    let json: Value = serde_json::from_str(&body)
        .map_err(|error| AppError::InvalidInput(format!("解析 AI 响应失败：{error}")))?;
    let translated = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| {
            AppError::InvalidInput(
                "AI 响应格式异常，未找到 choices[0].message.content".to_string(),
            )
        })?
        .trim()
        .to_string();

    Ok(TranslateResponse {
        source_text: text.to_string(),
        target_text: translated,
        engine: TranslationEngine::Ai,
    })
}

async fn translate_google(
    config: &AppConfig,
    text: &str,
    target_lang: &str,
) -> AppResult<TranslateResponse> {
    let mut builder = reqwest::Client::builder().timeout(Duration::from_secs(15));
    let proxy_url = config.translation_proxy.trim();
    if !proxy_url.is_empty() {
        let proxy = reqwest::Proxy::all(proxy_url)
            .map_err(|error| AppError::InvalidInput(format!("代理配置无效（{proxy_url}）：{error}")))?;
        builder = builder.proxy(proxy);
    }

    let client = builder
        .build()
        .map_err(|error| AppError::InvalidInput(format!("创建 HTTP 客户端失败：{error}")))?;

    let response = client
        .get("https://translate.googleapis.com/translate_a/single")
        .query(&[
            ("client", "gtx"),
            ("sl", "auto"),
            ("tl", target_lang),
            ("dt", "t"),
            ("q", text),
        ])
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
        .map_err(format_google_error)?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| AppError::InvalidInput(format!("读取 Google 响应失败：{error}")))?;

    if !status.is_success() {
        return Err(AppError::InvalidInput(format!(
            "Google 翻译 HTTP {}：{}",
            status.as_u16(),
            truncate_for_error(&body)
        )));
    }

    let json: Value = serde_json::from_str(&body)
        .map_err(|error| AppError::InvalidInput(format!("解析 Google 响应失败：{error}")))?;
    let translated = json[0][0][0]
        .as_str()
        .ok_or_else(|| AppError::InvalidInput("Google 响应格式异常".to_string()))?
        .to_string();

    Ok(TranslateResponse {
        source_text: text.to_string(),
        target_text: translated,
        engine: TranslationEngine::Google,
    })
}

fn format_google_error(error: reqwest::Error) -> AppError {
    if error.is_connect() {
        AppError::InvalidInput("Google 翻译连接失败，请检查网络或代理配置".to_string())
    } else if error.is_timeout() {
        AppError::InvalidInput("Google 翻译请求超时".to_string())
    } else {
        AppError::InvalidInput(format!("Google 翻译请求失败：{error}"))
    }
}

fn truncate_for_error(text: &str) -> String {
    text.chars().take(120).collect()
}
