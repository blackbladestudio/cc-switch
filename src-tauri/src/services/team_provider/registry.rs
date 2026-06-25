use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use crate::provider::{
    ClaudeDesktopModelRoute, ClaudeModelConfig, CodexModelConfig, GeminiModelConfig, ProviderMeta,
};

pub const REGISTRY_SCHEMA_VERSION: i64 = 1;

/// 团队 Provider Registry 文档
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamProviderRegistry {
    pub version: i64,
    pub team_id: String,
    pub updated_at: String,
    pub providers: Vec<TeamRegistryEntry>,
}

/// Registry 中的单条 provider 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamRegistryEntry {
    pub id: String,
    pub name: String,
    pub apps: Vec<String>,
    pub base_url: String,
    pub api_key_policy: String,
    #[serde(default)]
    pub models: TeamRegistryModels,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ProviderMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TeamRegistryModels {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude: Option<ClaudeModelConfig>,
    #[serde(rename = "claudeDesktop", skip_serializing_if = "Option::is_none")]
    pub claude_desktop: Option<TeamClaudeDesktopConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codex: Option<CodexModelConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gemini: Option<GeminiModelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamClaudeDesktopConfig {
    #[serde(default = "default_claude_desktop_mode")]
    pub mode: String,
    #[serde(
        default,
        rename = "modelRoutes",
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub model_routes: HashMap<String, ClaudeDesktopModelRoute>,
}

impl Default for TeamClaudeDesktopConfig {
    fn default() -> Self {
        Self {
            mode: default_claude_desktop_mode(),
            model_routes: HashMap::new(),
        }
    }
}

fn default_claude_desktop_mode() -> String {
    "proxy".to_string()
}

impl TeamRegistryEntry {
    pub fn has_app(&self, app: &str) -> bool {
        self.apps
            .iter()
            .any(|value| value.eq_ignore_ascii_case(app))
    }
}

pub fn team_local_provider_id(app: &str, registry_id: &str) -> String {
    format!("team-{}-{}", app, registry_id)
}
