use std::{sync::Mutex, time::Duration};

use serde_json::Value;

use crate::{
    error::{AppError, AppResult},
    models::{AppConfig, TranslateResponse, TranslationEngine},
};

static AI_CLIENT: Mutex<Option<(String, reqwest::Client)>> = Mutex::new(None);
static GOOGLE_CLIENT: Mutex<Option<(String, reqwest::Client)>> = Mutex::new(None);

fn cached_translation_client(
    cache: &Mutex<Option<(String, reqwest::Client)>>,
    proxy_url: &str,
    timeout: Duration,
) -> AppResult<reqwest::Client> {
    let proxy_url = proxy_url.trim();
    let mut cached = cache.lock().unwrap_or_else(|error| error.into_inner());
    if let Some((key, client)) = cached.as_ref() {
        if key == proxy_url {
            return Ok(client.clone());
        }
    }
    let mut builder = reqwest::Client::builder().timeout(timeout);
    if !proxy_url.is_empty() {
        let proxy = reqwest::Proxy::all(proxy_url).map_err(|error| {
            AppError::InvalidInput(format!("代理配置无效（{proxy_url}）：{error}"))
        })?;
        builder = builder.proxy(proxy);
    }
    let client = builder
        .build()
        .map_err(|error| AppError::InvalidInput(format!("创建 HTTP 客户端失败：{error}")))?;
    *cached = Some((proxy_url.to_string(), client.clone()));
    Ok(client)
}

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
    let model = if model.is_empty() {
        "gpt-4o-mini"
    } else {
        model
    };
    let prompt = format!(
        "Translate the following text to {target_lang}. Only output the translated text, nothing else.\n\nText: {text}",
    );

    let client = cached_translation_client(&AI_CLIENT, "", Duration::from_secs(30))?;

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
            AppError::InvalidInput("AI 响应格式异常，未找到 choices[0].message.content".to_string())
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
    let client = cached_translation_client(
        &GOOGLE_CLIENT,
        &config.translation_proxy,
        Duration::from_secs(15),
    )?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    #[tokio::test]
    async fn cached_client_reuses_connections_and_rebuilds_for_proxy_changes() {
        tokio::time::timeout(Duration::from_secs(5), async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let server = tokio::spawn(async move {
                for (connection, requests) in [2, 1].into_iter().enumerate() {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    for request in 0..requests {
                        let mut header = Vec::new();
                        while !header.ends_with(b"\r\n\r\n") {
                            header.push(stream.read_u8().await.unwrap());
                            assert!(header.len() < 8192);
                        }
                        let header = String::from_utf8(header).unwrap();
                        let expected = if connection == 0 {
                            format!("GET /{request} HTTP/1.1")
                        } else {
                            "GET http://translation.invalid/proxy HTTP/1.1".to_string()
                        };
                        assert!(header.starts_with(&expected), "{header}");
                        stream
                            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                            .await
                            .unwrap();
                    }
                }
            });
            let cache = Mutex::new(None);
            for request in 0..2 {
                let client =
                    cached_translation_client(&cache, "  ", Duration::from_secs(2)).unwrap();
                assert_eq!(
                    client
                        .get(format!("http://{address}/{request}"))
                        .send()
                        .await
                        .unwrap()
                        .text()
                        .await
                        .unwrap(),
                    "ok"
                );
            }
            assert!(cached_translation_client(&cache, "http://[", Duration::from_secs(2)).is_err());
            assert_eq!(cache.lock().unwrap().as_ref().unwrap().0, "");
            let proxy = format!("http://{address}");
            let client = cached_translation_client(&cache, &proxy, Duration::from_secs(2)).unwrap();
            assert_eq!(
                client
                    .get("http://translation.invalid/proxy")
                    .send()
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap(),
                "ok"
            );
            assert_eq!(cache.lock().unwrap().as_ref().unwrap().0, proxy);
            server.await.unwrap();
            cached_translation_client(&cache, "", Duration::from_secs(2)).unwrap();
            assert_eq!(cache.lock().unwrap().as_ref().unwrap().0, "");
        })
        .await
        .expect("connection reuse or proxy selection failed");
    }
}
