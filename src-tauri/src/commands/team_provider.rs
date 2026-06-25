use tauri::{AppHandle, Emitter, State};

use crate::services::team_provider::{TeamProviderRegistry, TeamProviderService};
use crate::settings::{TeamProviderSyncSettings, TeamSyncApplySummary, TeamSyncStatus};
use crate::store::AppState;

#[tauri::command]
pub fn get_team_sync_settings() -> Result<Option<TeamProviderSyncSettings>, String> {
    Ok(crate::services::team_provider::auto_sync::get_sync_settings())
}

#[tauri::command]
pub fn save_team_sync_settings(settings: Option<TeamProviderSyncSettings>) -> Result<(), String> {
    crate::services::team_provider::auto_sync::save_sync_settings(settings)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_team_sync_status() -> Result<TeamSyncStatus, String> {
    Ok(crate::services::team_provider::auto_sync::get_sync_status())
}

#[tauri::command]
pub async fn fetch_team_registry(source_url: String) -> Result<TeamProviderRegistry, String> {
    TeamProviderService::fetch_registry_only(&source_url)
        .await
        .map(|(registry, _etag)| registry)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn apply_team_registry(
    app: AppHandle,
    state: State<'_, AppState>,
    source_url: String,
) -> Result<TeamSyncApplySummary, String> {
    let summary = TeamProviderService::apply_from_source(&state.db, &source_url)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app.emit(
        "team-provider-synced",
        serde_json::json!({
            "source": "manual",
            "summary": summary,
        }),
    );

    Ok(summary)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveTeamSyncConflictRequest {
    pub provider_id: String,
    pub app: String,
    pub accept_team: bool,
    pub source_url: String,
}

#[tauri::command]
pub async fn resolve_team_sync_conflict(
    state: State<'_, AppState>,
    request: ResolveTeamSyncConflictRequest,
) -> Result<(), String> {
    let registry = TeamProviderService::fetch_registry_only(&request.source_url)
        .await
        .map(|(registry, _etag)| registry)
        .map_err(|e| e.to_string())?;

    TeamProviderService::resolve_conflict(
        &state.db,
        &request.provider_id,
        &request.app,
        request.accept_team,
        &registry,
        &request.source_url,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cleanup_removed_team_providers(state: State<'_, AppState>) -> Result<u32, String> {
    TeamProviderService::cleanup_removed(&state.db).map_err(|e| e.to_string())
}
