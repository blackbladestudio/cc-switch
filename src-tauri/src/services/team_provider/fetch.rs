use std::time::Duration;

use reqwest::header::{ETAG, IF_NONE_MATCH};

use crate::error::AppError;

use super::registry::TeamProviderRegistry;

const FETCH_TIMEOUT_SECS: u64 = 30;

pub struct FetchResult {
    pub registry: Option<TeamProviderRegistry>,
    pub etag: Option<String>,
    pub not_modified: bool,
}

pub async fn fetch_registry(
    source_url: &str,
    previous_etag: Option<&str>,
) -> Result<FetchResult, AppError> {
    let url = source_url.trim();
    if url.is_empty() {
        return Err(AppError::Message("团队 Registry URL 不能为空".into()));
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::Message(
            "团队 Registry URL 必须以 http:// 或 https:// 开头".into(),
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::Message(format!("创建 HTTP 客户端失败: {e}")))?;

    let mut request = client.get(url).header("Accept", "application/json");
    if let Some(etag) = previous_etag.filter(|value| !value.is_empty()) {
        request = request.header(IF_NONE_MATCH, etag);
    }

    let response = request
        .send()
        .await
        .map_err(|e| AppError::Message(format!("拉取团队 Registry 失败: {e}")))?;

    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(FetchResult {
            registry: None,
            etag: previous_etag.map(str::to_string),
            not_modified: true,
        });
    }

    if !response.status().is_success() {
        return Err(AppError::Message(format!(
            "拉取团队 Registry 失败: HTTP {}",
            response.status()
        )));
    }

    let etag = response
        .headers()
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let raw: serde_json::Value = response
        .json()
        .await
        .map_err(|e| AppError::Message(format!("解析团队 Registry JSON 失败: {e}")))?;

    if super::validate::contains_plaintext_secret(&raw) {
        return Err(AppError::Message(
            "Registry 不能包含明文 API Key 或凭证字段".into(),
        ));
    }

    let registry: TeamProviderRegistry = serde_json::from_value(raw)
        .map_err(|e| AppError::Message(format!("解析团队 Registry 结构失败: {e}")))?;

    Ok(FetchResult {
        registry: Some(registry),
        etag,
        not_modified: false,
    })
}
