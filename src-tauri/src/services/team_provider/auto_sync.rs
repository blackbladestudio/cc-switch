use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{sleep, Instant};

use crate::error::AppError;
use crate::services::team_provider::TeamProviderService;
use crate::settings::{self, TeamProviderSyncSettings, TeamSyncApplySummary, TeamSyncStatus};
use crate::store::AppState;

const MIN_INTERVAL_MINUTES: u64 = 5;
const MAX_BACKOFF_MINUTES: u64 = 60;

pub fn start_worker(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut backoff_minutes = 0u64;
        loop {
            let settings = settings::get_team_provider_sync_settings();
            let interval_minutes = settings
                .as_ref()
                .filter(|sync| sync.enabled)
                .map(|sync| sync.auto_sync_interval_minutes as u64)
                .unwrap_or(0);

            if interval_minutes == 0 {
                sleep(Duration::from_secs(30)).await;
                continue;
            }

            let wait_minutes = interval_minutes.max(MIN_INTERVAL_MINUTES);
            if backoff_minutes > 0 {
                sleep(Duration::from_secs(backoff_minutes * 60)).await;
            } else {
                sleep(Duration::from_secs(wait_minutes * 60)).await;
            }

            let Some(sync_settings) = settings::get_team_provider_sync_settings() else {
                continue;
            };
            if !sync_settings.enabled || sync_settings.source_url.trim().is_empty() {
                continue;
            }

            let state = app.state::<AppState>();
            let source_url = sync_settings.source_url.clone();
            let started = Instant::now();
            match TeamProviderService::apply_from_source(&state.db, &source_url).await {
                Ok(summary) => {
                    backoff_minutes = 0;
                    emit_team_provider_synced(&app, &summary, None);
                    log::info!(
                        "[TeamProvider] auto sync completed in {:?}",
                        started.elapsed()
                    );
                }
                Err(err) => {
                    backoff_minutes = if backoff_minutes == 0 {
                        MIN_INTERVAL_MINUTES
                    } else {
                        (backoff_minutes * 2).min(MAX_BACKOFF_MINUTES)
                    };
                    persist_auto_sync_error(&err);
                    emit_team_provider_synced(&app, &TeamSyncApplySummary::default(), Some(&err));
                    log::warn!("[TeamProvider] auto sync failed: {err}");
                }
            }
        }
    });
}

fn persist_auto_sync_error(error: &AppError) {
    let mut status = settings::get_team_provider_sync_settings()
        .map(|sync| sync.status)
        .unwrap_or_default();
    status.last_error = Some(error.to_string());
    let _ = settings::update_team_sync_status(status);
}

fn emit_team_provider_synced(
    app: &AppHandle,
    summary: &TeamSyncApplySummary,
    error: Option<&AppError>,
) {
    let payload = serde_json::json!({
        "source": "auto",
        "summary": summary,
        "error": error.map(|err| err.to_string()),
    });
    if let Err(err) = app.emit("team-provider-synced", payload) {
        log::debug!("[TeamProvider] failed to emit sync event: {err}");
    }
}

pub fn get_sync_settings() -> Option<TeamProviderSyncSettings> {
    settings::get_team_provider_sync_settings()
}

pub fn save_sync_settings(settings: Option<TeamProviderSyncSettings>) -> Result<(), AppError> {
    settings::set_team_provider_sync_settings(settings)
}

pub fn get_sync_status() -> TeamSyncStatus {
    settings::get_team_provider_sync_settings()
        .map(|sync| sync.status)
        .unwrap_or_default()
}
