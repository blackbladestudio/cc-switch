use regex::Regex;
use std::sync::LazyLock;

use crate::error::AppError;

use super::registry::{TeamProviderRegistry, TeamRegistryEntry, REGISTRY_SCHEMA_VERSION};

static PROVIDER_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9][a-z0-9-_]{0,63}$").expect("valid provider id regex"));

const SUPPORTED_APPS: [&str; 4] = ["claude", "claude-desktop", "codex", "gemini"];

pub fn validate_registry(registry: &TeamProviderRegistry) -> Result<(), AppError> {
    if registry.version != REGISTRY_SCHEMA_VERSION {
        return Err(AppError::Message(format!(
            "不支持的 Registry 版本: {}（当前仅支持 {}）",
            registry.version, REGISTRY_SCHEMA_VERSION
        )));
    }
    if registry.team_id.trim().is_empty() {
        return Err(AppError::Message("Registry teamId 不能为空".into()));
    }
    if registry.updated_at.trim().is_empty() {
        return Err(AppError::Message("Registry updatedAt 不能为空".into()));
    }
    if registry.providers.is_empty() {
        return Err(AppError::Message("Registry providers 不能为空".into()));
    }

    let mut seen_ids = std::collections::HashSet::new();
    for entry in &registry.providers {
        validate_entry(entry)?;
        if !seen_ids.insert(entry.id.clone()) {
            return Err(AppError::Message(format!(
                "Registry 中存在重复的 provider id: {}",
                entry.id
            )));
        }
    }

    Ok(())
}

pub fn validate_entry(entry: &TeamRegistryEntry) -> Result<(), AppError> {
    if !PROVIDER_ID_RE.is_match(&entry.id) {
        return Err(AppError::Message(format!(
            "无效的 provider id: {}",
            entry.id
        )));
    }
    if entry.name.trim().is_empty() {
        return Err(AppError::Message(format!(
            "provider {} 的 name 不能为空",
            entry.id
        )));
    }
    if entry.apps.is_empty() {
        return Err(AppError::Message(format!(
            "provider {} 的 apps 不能为空",
            entry.id
        )));
    }
    for app in &entry.apps {
        if !SUPPORTED_APPS
            .iter()
            .any(|supported| supported.eq_ignore_ascii_case(app))
        {
            return Err(AppError::Message(format!(
                "provider {} 包含不支持的 app: {app}",
                entry.id
            )));
        }
    }
    if entry.base_url.trim().is_empty()
        || (!entry.base_url.starts_with("http://") && !entry.base_url.starts_with("https://"))
    {
        return Err(AppError::Message(format!(
            "provider {} 的 baseUrl 必须是合法的 HTTP(S) URL",
            entry.id
        )));
    }
    if entry.api_key_policy != "local_required" {
        return Err(AppError::Message(format!(
            "provider {} 的 apiKeyPolicy 仅支持 local_required",
            entry.id
        )));
    }
    if entry.has_app("claude-desktop") {
        let mode = entry
            .models
            .claude_desktop
            .as_ref()
            .map(|config| config.mode.as_str())
            .unwrap_or("proxy");
        if mode != "direct" && mode != "proxy" {
            return Err(AppError::Message(format!(
                "provider {} 的 claudeDesktop.mode 仅支持 direct 或 proxy",
                entry.id
            )));
        }
    }

    let serialized = serde_json::to_value(entry).map_err(|e| AppError::Message(e.to_string()))?;
    if contains_plaintext_secret(&serialized) {
        return Err(AppError::Message(format!(
            "provider {} 不能在 Registry 中包含明文 API Key",
            entry.id
        )));
    }

    Ok(())
}

pub fn contains_plaintext_secret(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let lower = key.to_ascii_lowercase();
                if (lower == "apikey"
                    || lower == "api_key"
                    || lower.ends_with("_api_key")
                    || lower.contains("auth_token")
                    || lower == "openai_api_key"
                    || lower == "anthropic_auth_token"
                    || lower == "anthropic_api_key"
                    || lower == "gemini_api_key")
                    && child.as_str().is_some_and(|s| !s.trim().is_empty())
                {
                    return true;
                }
                if contains_plaintext_secret(child) {
                    return true;
                }
            }
            false
        }
        serde_json::Value::Array(items) => items.iter().any(contains_plaintext_secret),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(id: &str) -> TeamRegistryEntry {
        TeamRegistryEntry {
            id: id.to_string(),
            name: "Team NewAPI".to_string(),
            apps: vec!["claude".to_string()],
            base_url: "https://api.example.com/v1".to_string(),
            api_key_policy: "local_required".to_string(),
            models: Default::default(),
            website_url: None,
            notes: None,
            icon: None,
            icon_color: None,
            meta: None,
        }
    }

    #[test]
    fn rejects_invalid_provider_id() {
        let mut entry = sample_entry("Bad ID");
        assert!(validate_entry(&entry).is_err());

        entry.id = "team-newapi".to_string();
        assert!(validate_entry(&entry).is_ok());
    }

    #[test]
    fn rejects_plaintext_api_key_in_raw_json() {
        let raw = serde_json::json!({
            "providers": [{
                "id": "newapi",
                "apiKey": "sk-secret"
            }]
        });
        assert!(contains_plaintext_secret(&raw));
    }
}
