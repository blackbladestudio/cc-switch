use serde_json::{Map, Value};

use crate::app_config::AppType;
use crate::provider::Provider;

pub fn extract_secrets(provider: &Provider, app: &AppType) -> Value {
    match app {
        AppType::Claude | AppType::ClaudeDesktop => {
            let mut env = Map::new();
            if let Some(settings) = provider.settings_config.as_object() {
                if let Some(env_obj) = settings.get("env").and_then(|v| v.as_object()) {
                    for key in [
                        "ANTHROPIC_AUTH_TOKEN",
                        "ANTHROPIC_API_KEY",
                        "OPENROUTER_API_KEY",
                        "GOOGLE_API_KEY",
                    ] {
                        if let Some(value) = env_obj.get(key).and_then(|v| v.as_str()) {
                            if !value.is_empty() {
                                env.insert(key.to_string(), Value::String(value.to_string()));
                            }
                        }
                    }
                }
            }
            Value::Object(env)
        }
        AppType::Codex => {
            let mut auth = Map::new();
            if let Some(settings) = provider.settings_config.as_object() {
                if let Some(auth_obj) = settings.get("auth").and_then(|v| v.as_object()) {
                    if let Some(value) = auth_obj
                        .get("OPENAI_API_KEY")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                    {
                        auth.insert(
                            "OPENAI_API_KEY".to_string(),
                            Value::String(value.to_string()),
                        );
                    }
                }
            }
            Value::Object(auth)
        }
        AppType::Gemini => {
            let mut env = Map::new();
            if let Some(settings) = provider.settings_config.as_object() {
                if let Some(env_obj) = settings.get("env").and_then(|v| v.as_object()) {
                    for key in ["GEMINI_API_KEY", "GOOGLE_API_KEY"] {
                        if let Some(value) = env_obj.get(key).and_then(|v| v.as_str()) {
                            if !value.is_empty() {
                                env.insert(key.to_string(), Value::String(value.to_string()));
                            }
                        }
                    }
                }
            }
            Value::Object(env)
        }
        _ => Value::Null,
    }
}

pub fn restore_secrets(target: &mut Provider, secrets: &Value, app: &AppType) {
    let Some(settings) = target.settings_config.as_object_mut() else {
        return;
    };

    match app {
        AppType::Claude | AppType::ClaudeDesktop => {
            if let Some(secret_env) = secrets.as_object() {
                if secret_env.is_empty() {
                    return;
                }
                let env = settings
                    .entry("env".to_string())
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Some(env_map) = env.as_object_mut() {
                    for (key, value) in secret_env {
                        env_map.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        AppType::Codex => {
            if let Some(secret_auth) = secrets.as_object() {
                if secret_auth.is_empty() {
                    return;
                }
                let auth = settings
                    .entry("auth".to_string())
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Some(auth_map) = auth.as_object_mut() {
                    for (key, value) in secret_auth {
                        auth_map.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        AppType::Gemini => {
            if let Some(secret_env) = secrets.as_object() {
                if secret_env.is_empty() {
                    return;
                }
                let env = settings
                    .entry("env".to_string())
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Some(env_map) = env.as_object_mut() {
                    for (key, value) in secret_env {
                        env_map.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        _ => {}
    }
}

pub fn merge_team_provider(
    existing: Option<&Provider>,
    mut incoming: Provider,
    app: &AppType,
) -> Provider {
    let secrets = existing
        .map(|provider| extract_secrets(provider, app))
        .unwrap_or(Value::Null);

    if let Some(existing) = existing {
        if existing.notes.is_some() {
            incoming.notes = existing.notes.clone();
        }
        if existing.sort_index.is_some() {
            incoming.sort_index = existing.sort_index;
        }
        if existing.created_at.is_some() {
            incoming.created_at = existing.created_at;
        }
    }

    restore_secrets(&mut incoming, &secrets, app);
    incoming
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Provider;

    #[test]
    fn preserves_existing_api_key_on_merge() {
        let existing = Provider {
            id: "team-claude-newapi".to_string(),
            name: "Old".to_string(),
            settings_config: serde_json::json!({
                "env": {
                    "ANTHROPIC_BASE_URL": "https://old.example.com/v1",
                    "ANTHROPIC_AUTH_TOKEN": "sk-local"
                }
            }),
            website_url: None,
            category: None,
            created_at: None,
            sort_index: None,
            notes: Some("mine".to_string()),
            meta: None,
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        };

        let incoming = Provider {
            id: existing.id.clone(),
            name: "Team".to_string(),
            settings_config: serde_json::json!({
                "env": {
                    "ANTHROPIC_BASE_URL": "https://new.example.com/v1",
                    "ANTHROPIC_AUTH_TOKEN": ""
                }
            }),
            website_url: None,
            category: None,
            created_at: None,
            sort_index: None,
            notes: None,
            meta: None,
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        };

        let merged = merge_team_provider(Some(&existing), incoming, &AppType::Claude);
        assert_eq!(
            merged.settings_config["env"]["ANTHROPIC_BASE_URL"],
            "https://new.example.com/v1"
        );
        assert_eq!(
            merged.settings_config["env"]["ANTHROPIC_AUTH_TOKEN"],
            "sk-local"
        );
        assert_eq!(merged.notes.as_deref(), Some("mine"));
    }
}
