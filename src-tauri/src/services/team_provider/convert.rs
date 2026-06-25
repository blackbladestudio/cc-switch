use crate::app_config::AppType;
use crate::provider::{ClaudeDesktopMode, Provider};

use super::registry::{team_local_provider_id, TeamRegistryEntry};

impl TeamRegistryEntry {
    pub fn to_claude_provider(&self) -> Option<Provider> {
        if !self.has_app("claude") {
            return None;
        }

        let models = self.models.claude.as_ref();
        let model = models
            .and_then(|m| m.model.clone())
            .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());
        let haiku = models
            .and_then(|m| m.haiku_model.clone())
            .unwrap_or_else(|| model.clone());
        let sonnet = models
            .and_then(|m| m.sonnet_model.clone())
            .unwrap_or_else(|| model.clone());
        let opus = models
            .and_then(|m| m.opus_model.clone())
            .unwrap_or_else(|| model.clone());

        let settings_config = serde_json::json!({
            "env": {
                "ANTHROPIC_BASE_URL": self.base_url,
                "ANTHROPIC_AUTH_TOKEN": "",
                "ANTHROPIC_MODEL": model,
                "ANTHROPIC_DEFAULT_HAIKU_MODEL": haiku,
                "ANTHROPIC_DEFAULT_SONNET_MODEL": sonnet,
                "ANTHROPIC_DEFAULT_OPUS_MODEL": opus,
            }
        });

        Some(Provider {
            id: team_local_provider_id("claude", &self.id),
            name: self.name.clone(),
            settings_config,
            website_url: self.website_url.clone(),
            category: Some("aggregator".to_string()),
            created_at: None,
            sort_index: None,
            notes: self.notes.clone(),
            meta: self.meta.clone(),
            icon: self.icon.clone(),
            icon_color: self.icon_color.clone(),
            in_failover_queue: false,
        })
    }

    pub fn to_codex_provider(&self) -> Option<Provider> {
        if !self.has_app("codex") {
            return None;
        }

        let models = self.models.codex.as_ref();
        let model = models
            .and_then(|m| m.model.clone())
            .unwrap_or_else(|| "gpt-4o".to_string());
        let reasoning_effort = models
            .and_then(|m| m.reasoning_effort.clone())
            .unwrap_or_else(|| "high".to_string());

        let base_trimmed = self.base_url.trim_end_matches('/');
        let origin_only = match base_trimmed.split_once("://") {
            Some((_scheme, rest)) => !rest.contains('/'),
            None => !base_trimmed.contains('/'),
        };
        let codex_base_url = if base_trimmed.ends_with("/v1") {
            base_trimmed.to_string()
        } else if origin_only {
            format!("{base_trimmed}/v1")
        } else {
            base_trimmed.to_string()
        };

        let config_toml = format!(
            r#"model_provider = "custom"
model = "{model}"
model_reasoning_effort = "{reasoning_effort}"
disable_response_storage = true

[model_providers.custom]
name = "NewAPI"
base_url = "{codex_base_url}"
wire_api = "responses"
requires_openai_auth = true"#
        );

        let settings_config = serde_json::json!({
            "auth": {
                "OPENAI_API_KEY": ""
            },
            "config": config_toml
        });

        Some(Provider {
            id: team_local_provider_id("codex", &self.id),
            name: self.name.clone(),
            settings_config,
            website_url: self.website_url.clone(),
            category: Some("aggregator".to_string()),
            created_at: None,
            sort_index: None,
            notes: self.notes.clone(),
            meta: self.meta.clone(),
            icon: self.icon.clone(),
            icon_color: self.icon_color.clone(),
            in_failover_queue: false,
        })
    }

    pub fn to_claude_desktop_provider(&self) -> Option<Provider> {
        if !self.has_app("claude-desktop") {
            return None;
        }

        let desktop = self.models.claude_desktop.clone().unwrap_or_default();
        let mode = match desktop.mode.as_str() {
            "direct" => ClaudeDesktopMode::Direct,
            _ => ClaudeDesktopMode::Proxy,
        };

        let claude_models = self.models.claude.as_ref();
        let fallback_model = claude_models
            .and_then(|m| m.model.clone())
            .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

        let settings_config = serde_json::json!({
            "env": {
                "ANTHROPIC_BASE_URL": self.base_url,
                "ANTHROPIC_AUTH_TOKEN": "",
                "ANTHROPIC_MODEL": fallback_model,
            }
        });

        let mut meta = self.meta.clone().unwrap_or_default();
        meta.claude_desktop_mode = Some(mode);
        if !desktop.model_routes.is_empty() {
            meta.claude_desktop_model_routes = desktop.model_routes;
        }

        Some(Provider {
            id: team_local_provider_id("claude-desktop", &self.id),
            name: self.name.clone(),
            settings_config,
            website_url: self.website_url.clone(),
            category: Some("aggregator".to_string()),
            created_at: None,
            sort_index: None,
            notes: self.notes.clone(),
            meta: Some(meta),
            icon: self.icon.clone(),
            icon_color: self.icon_color.clone(),
            in_failover_queue: false,
        })
    }

    pub fn to_gemini_provider(&self) -> Option<Provider> {
        if !self.has_app("gemini") {
            return None;
        }

        let models = self.models.gemini.as_ref();
        let model = models
            .and_then(|m| m.model.clone())
            .unwrap_or_else(|| "gemini-2.5-pro".to_string());

        let settings_config = serde_json::json!({
            "env": {
                "GOOGLE_GEMINI_BASE_URL": self.base_url,
                "GEMINI_API_KEY": "",
                "GEMINI_MODEL": model,
            }
        });

        Some(Provider {
            id: team_local_provider_id("gemini", &self.id),
            name: self.name.clone(),
            settings_config,
            website_url: self.website_url.clone(),
            category: Some("aggregator".to_string()),
            created_at: None,
            sort_index: None,
            notes: self.notes.clone(),
            meta: self.meta.clone(),
            icon: self.icon.clone(),
            icon_color: self.icon_color.clone(),
            in_failover_queue: false,
        })
    }

    pub fn to_provider_for_app(&self, app: &str) -> Option<Provider> {
        match app {
            "claude" => self.to_claude_provider(),
            "claude-desktop" => self.to_claude_desktop_provider(),
            "codex" => self.to_codex_provider(),
            "gemini" => self.to_gemini_provider(),
            _ => None,
        }
    }
}

pub fn locked_fields_for_app(app: &str) -> Vec<String> {
    match app {
        "claude" => vec![
            "settingsConfig.env.ANTHROPIC_BASE_URL".to_string(),
            "settingsConfig.env.ANTHROPIC_MODEL".to_string(),
            "meta.apiFormat".to_string(),
        ],
        "claude-desktop" => vec![
            "settingsConfig.env.ANTHROPIC_BASE_URL".to_string(),
            "settingsConfig.env.ANTHROPIC_MODEL".to_string(),
            "meta.claudeDesktopMode".to_string(),
            "meta.claudeDesktopModelRoutes".to_string(),
            "meta.apiFormat".to_string(),
            "meta.modelApiFormats".to_string(),
            "meta.modelApiOverrides".to_string(),
        ],
        "codex" => vec![
            "settingsConfig.config".to_string(),
            "meta.apiFormat".to_string(),
        ],
        "gemini" => vec![
            "settingsConfig.env.GOOGLE_GEMINI_BASE_URL".to_string(),
            "settingsConfig.env.GEMINI_MODEL".to_string(),
        ],
        _ => Vec::new(),
    }
}

pub fn app_type_from_str(app: &str) -> Option<AppType> {
    match app {
        "claude" => Some(AppType::Claude),
        "claude-desktop" => Some(AppType::ClaudeDesktop),
        "codex" => Some(AppType::Codex),
        "gemini" => Some(AppType::Gemini),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_registry_entry_to_team_provider_ids() {
        let entry = TeamRegistryEntry {
            id: "newapi".to_string(),
            name: "Team".to_string(),
            apps: vec![
                "claude".to_string(),
                "claude-desktop".to_string(),
                "codex".to_string(),
                "gemini".to_string(),
            ],
            base_url: "https://api.example.com/v1".to_string(),
            api_key_policy: "local_required".to_string(),
            models: Default::default(),
            website_url: None,
            notes: None,
            icon: None,
            icon_color: None,
            meta: None,
        };

        assert_eq!(entry.to_claude_provider().unwrap().id, "team-claude-newapi");
        assert_eq!(
            entry.to_claude_desktop_provider().unwrap().id,
            "team-claude-desktop-newapi"
        );
        assert_eq!(entry.to_codex_provider().unwrap().id, "team-codex-newapi");
        assert_eq!(entry.to_gemini_provider().unwrap().id, "team-gemini-newapi");
    }
}
