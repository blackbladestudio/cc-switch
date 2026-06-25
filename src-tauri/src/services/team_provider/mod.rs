pub mod auto_sync;
mod convert;
mod fetch;
mod merge;
mod registry;
mod validate;

pub use registry::TeamProviderRegistry;

use std::collections::HashSet;

use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::database::Database;
use crate::error::AppError;
use crate::provider::{Provider, TeamManagedMeta};
use crate::settings::{self, TeamSyncApplySummary, TeamSyncConflict};

use convert::{app_type_from_str, locked_fields_for_app};
use merge::merge_team_provider;
use registry::team_local_provider_id;

const SUPPORTED_APPS: [&str; 4] = ["claude", "claude-desktop", "codex", "gemini"];

pub struct TeamProviderService;

impl TeamProviderService {
    pub async fn fetch_registry_only(
        source_url: &str,
    ) -> Result<(TeamProviderRegistry, Option<String>), AppError> {
        let result = fetch::fetch_registry(source_url, None).await?;
        if result.not_modified {
            return Err(AppError::Message("Registry 未变更 (304)".into()));
        }
        let registry = result
            .registry
            .ok_or_else(|| AppError::Message("未获取到 Registry 内容".into()))?;
        validate::validate_registry(&registry)?;
        Ok((registry, result.etag))
    }

    pub async fn apply_registry(
        db: &Database,
        registry: &TeamProviderRegistry,
        source_url: &str,
        remote_etag: Option<String>,
    ) -> Result<TeamSyncApplySummary, AppError> {
        let mut summary = TeamSyncApplySummary::default();
        let now = Utc::now().timestamp_millis();
        let registry_ids: HashSet<String> =
            registry.providers.iter().map(|p| p.id.clone()).collect();

        for entry in &registry.providers {
            for app in SUPPORTED_APPS {
                if !entry.has_app(app) {
                    if let Some(existing) =
                        db.get_provider_by_id(&team_local_provider_id(app, &entry.id), app)?
                    {
                        if mark_removed_if_team_managed(&existing, db, app, &mut summary)? {
                            summary.removed += 1;
                        }
                    }
                    continue;
                }

                let Some(incoming) = entry.to_provider_for_app(app) else {
                    continue;
                };
                let provider_id = incoming.id.clone();
                let existing = db.get_provider_by_id(&provider_id, app)?;

                if let Some(existing) = existing.as_ref() {
                    if should_report_conflict(existing, &incoming, app) {
                        summary.conflicts.push(TeamSyncConflict {
                            provider_id: provider_id.clone(),
                            app: app.to_string(),
                            registry_entry_id: entry.id.clone(),
                            message: Some("本地已修改团队托管字段".into()),
                        });
                        summary.skipped += 1;
                        continue;
                    }
                }

                let merged = merge_team_provider(
                    existing.as_ref(),
                    incoming,
                    &app_type_from_str(app).unwrap(),
                );
                let mut final_provider = merged;
                attach_team_managed_meta(
                    &mut final_provider,
                    registry,
                    source_url,
                    entry.id.as_str(),
                    app,
                    now,
                );

                let is_create = existing.is_none();
                db.save_provider(app, &final_provider)?;
                if is_create {
                    summary.created += 1;
                } else {
                    summary.updated += 1;
                }
            }
        }

        mark_missing_registry_ids(db, &registry_ids, &mut summary)?;

        let mut status = settings::get_team_provider_sync_settings()
            .map(|sync| sync.status)
            .unwrap_or_default();
        status.last_sync_at = Some(now);
        status.last_success_at = Some(now);
        status.last_error = None;
        status.last_registry_updated_at = Some(registry.updated_at.clone());
        status.last_remote_etag = remote_etag;
        status.last_summary = Some(summary.clone());
        status.pending_conflicts = summary.conflicts.clone();
        settings::update_team_sync_status(status)?;

        Ok(summary)
    }

    pub async fn apply_from_source(
        db: &Database,
        source_url: &str,
    ) -> Result<TeamSyncApplySummary, AppError> {
        let previous_etag = settings::get_team_provider_sync_settings()
            .and_then(|sync| sync.status.last_remote_etag.clone());
        let result = fetch::fetch_registry(source_url, previous_etag.as_deref()).await?;

        if result.not_modified {
            let summary = TeamSyncApplySummary::default();
            let mut status = settings::get_team_provider_sync_settings()
                .map(|sync| sync.status)
                .unwrap_or_default();
            status.last_sync_at = Some(Utc::now().timestamp_millis());
            status.last_error = None;
            status.last_remote_etag = result.etag;
            status.last_summary = Some(summary.clone());
            settings::update_team_sync_status(status)?;
            return Ok(summary);
        }

        let registry = result
            .registry
            .ok_or_else(|| AppError::Message("未获取到 Registry 内容".into()))?;
        validate::validate_registry(&registry)?;
        Self::apply_registry(db, &registry, source_url, result.etag).await
    }

    pub fn resolve_conflict(
        db: &Database,
        provider_id: &str,
        app: &str,
        accept_team: bool,
        registry: &TeamProviderRegistry,
        source_url: &str,
    ) -> Result<(), AppError> {
        let existing = db
            .get_provider_by_id(provider_id, app)?
            .ok_or_else(|| AppError::Message(format!("Provider {provider_id} 不存在")))?;

        let registry_entry_id = existing
            .meta
            .as_ref()
            .and_then(|meta| meta.team_managed.as_ref())
            .and_then(|tm| tm.registry_entry_id.clone())
            .ok_or_else(|| AppError::Message("不是团队托管 provider".into()))?;

        let entry = registry
            .providers
            .iter()
            .find(|item| item.id == registry_entry_id)
            .ok_or_else(|| AppError::Message("Registry 中找不到对应条目".into()))?;

        let app_type =
            app_type_from_str(app).ok_or_else(|| AppError::Message("不支持的 app".into()))?;

        if accept_team {
            let Some(incoming) = entry.to_provider_for_app(app) else {
                return Err(AppError::Message("Registry 条目未启用该 app".into()));
            };
            let merged = merge_team_provider(Some(&existing), incoming, &app_type);
            let mut final_provider = merged;
            attach_team_managed_meta(
                &mut final_provider,
                registry,
                source_url,
                &registry_entry_id,
                app,
                Utc::now().timestamp_millis(),
            );
            if let Some(meta) = final_provider.meta.as_mut() {
                if let Some(team_managed) = meta.team_managed.as_mut() {
                    team_managed.local_override = Some(false);
                }
            }
            db.save_provider(app, &final_provider)?;
        } else if let Some(mut provider) = existing.into() {
            let locked_fields = locked_fields_for_app(app);
            let local_hash = compute_locked_fields_hash(&provider, &locked_fields);
            let meta = provider.meta.get_or_insert_with(Default::default);
            let team_managed = meta.team_managed.get_or_insert_with(|| TeamManagedMeta {
                team_id: registry.team_id.clone(),
                registry_version: registry.version,
                ..Default::default()
            });
            team_managed.local_override = Some(true);
            team_managed.local_fields_hash = Some(local_hash);
            db.save_provider(app, &provider)?;
        }

        clear_pending_conflict(provider_id, app)?;

        Ok(())
    }

    pub fn cleanup_removed(db: &Database) -> Result<u32, AppError> {
        let mut deleted = 0u32;
        for app in SUPPORTED_APPS {
            let app_type = app_type_from_str(app).unwrap();
            let current_id = settings::get_effective_current_provider(db, &app_type)?;
            let providers = db.get_all_providers(app)?;
            for (id, provider) in providers {
                let is_removed = provider
                    .meta
                    .as_ref()
                    .and_then(|meta| meta.team_managed.as_ref())
                    .and_then(|tm| tm.removed)
                    .unwrap_or(false);
                if !is_removed {
                    continue;
                }
                if current_id.as_deref() == Some(id.as_str()) {
                    continue;
                }
                db.delete_provider(app, &id)?;
                deleted += 1;
            }
        }
        Ok(deleted)
    }
}

fn attach_team_managed_meta(
    provider: &mut Provider,
    registry: &TeamProviderRegistry,
    source_url: &str,
    registry_entry_id: &str,
    app: &str,
    now: i64,
) {
    let locked_fields = locked_fields_for_app(app);
    let hash = compute_locked_fields_hash(provider, &locked_fields);
    let meta = provider.meta.get_or_insert_with(Default::default);
    meta.team_managed = Some(TeamManagedMeta {
        team_id: registry.team_id.clone(),
        registry_version: registry.version,
        registry_updated_at: Some(registry.updated_at.clone()),
        source_url: Some(source_url.to_string()),
        locked_fields,
        last_synced_at: Some(now),
        local_override: Some(false),
        removed: Some(false),
        local_fields_hash: Some(hash),
        registry_entry_id: Some(registry_entry_id.to_string()),
    });
}

fn compute_locked_fields_hash(provider: &Provider, locked_fields: &[String]) -> String {
    let values: Vec<serde_json::Value> = locked_fields
        .iter()
        .map(|path| {
            serde_json::json!({
                "path": path,
                "value": locked_field_value(provider, path),
            })
        })
        .collect();
    let value = serde_json::json!({ "lockedFields": values });
    let bytes = serde_json::to_vec(&value).unwrap_or_default();
    let digest = Sha256::digest(bytes);
    format!("{:x}", digest)
}

fn locked_field_value(provider: &Provider, path: &str) -> serde_json::Value {
    if let Some(settings_path) = path.strip_prefix("settingsConfig.") {
        return value_at_dotted_path(&provider.settings_config, settings_path);
    }

    if let Some(meta_path) = path.strip_prefix("meta.") {
        let mut meta = provider.meta.clone().unwrap_or_default();
        meta.team_managed = None;
        let meta_value = serde_json::to_value(meta).unwrap_or(serde_json::Value::Null);
        return value_at_dotted_path(&meta_value, meta_path);
    }

    serde_json::Value::Null
}

fn value_at_dotted_path(value: &serde_json::Value, dotted_path: &str) -> serde_json::Value {
    let mut current = value;
    for part in dotted_path.split('.') {
        let Some(next) = current.get(part) else {
            return serde_json::Value::Null;
        };
        current = next;
    }
    current.clone()
}

fn should_report_conflict(existing: &Provider, incoming: &Provider, app: &str) -> bool {
    let Some(team_managed) = existing
        .meta
        .as_ref()
        .and_then(|meta| meta.team_managed.as_ref())
    else {
        return false;
    };
    if team_managed.local_override.unwrap_or(false) {
        return false;
    }
    if team_managed.removed.unwrap_or(false) {
        return false;
    }
    let locked_fields = locked_fields_for_app(app);
    let current_hash = compute_locked_fields_hash(existing, &locked_fields);
    if team_managed.local_fields_hash.as_deref() != Some(current_hash.as_str()) {
        return true;
    }
    let _ = incoming;
    false
}

fn clear_pending_conflict(provider_id: &str, app: &str) -> Result<(), AppError> {
    let mut status = settings::get_team_provider_sync_settings()
        .map(|sync| sync.status)
        .unwrap_or_default();
    status
        .pending_conflicts
        .retain(|conflict| !(conflict.provider_id == provider_id && conflict.app == app));
    if let Some(summary) = status.last_summary.as_mut() {
        summary
            .conflicts
            .retain(|conflict| !(conflict.provider_id == provider_id && conflict.app == app));
    }
    settings::update_team_sync_status(status)
}

fn mark_removed_if_team_managed(
    provider: &Provider,
    db: &Database,
    app: &str,
    summary: &mut TeamSyncApplySummary,
) -> Result<bool, AppError> {
    let _ = summary;
    let Some(meta) = provider.meta.as_ref() else {
        return Ok(false);
    };
    let Some(team_managed) = meta.team_managed.as_ref() else {
        return Ok(false);
    };
    if team_managed.removed.unwrap_or(false) {
        return Ok(false);
    }
    let mut updated = provider.clone();
    let meta = updated.meta.get_or_insert_with(Default::default);
    if let Some(tm) = meta.team_managed.as_mut() {
        tm.removed = Some(true);
    }
    db.save_provider(app, &updated)?;
    Ok(true)
}

fn mark_missing_registry_ids(
    db: &Database,
    registry_ids: &HashSet<String>,
    summary: &mut TeamSyncApplySummary,
) -> Result<(), AppError> {
    for app in SUPPORTED_APPS {
        let providers = db.get_all_providers(app)?;
        for (id, provider) in providers {
            let Some(entry_id) = provider
                .meta
                .as_ref()
                .and_then(|meta| meta.team_managed.as_ref())
                .and_then(|tm| tm.registry_entry_id.clone())
            else {
                continue;
            };
            if registry_ids.contains(&entry_id) {
                continue;
            }
            if mark_removed_if_team_managed(&provider, db, app, summary)? {
                summary.removed += 1;
            }
            let _ = id;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Provider;

    #[test]
    fn conflict_when_local_hash_changed() {
        let existing = Provider {
            id: "team-claude-newapi".to_string(),
            name: "Team".to_string(),
            settings_config: serde_json::json!({
                "env": { "ANTHROPIC_BASE_URL": "https://local.example.com/v1" }
            }),
            website_url: None,
            category: None,
            created_at: None,
            sort_index: None,
            notes: None,
            meta: Some(crate::provider::ProviderMeta {
                team_managed: Some(TeamManagedMeta {
                    team_id: "team-a".to_string(),
                    registry_version: 1,
                    local_fields_hash: Some("old-hash".to_string()),
                    local_override: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        };
        let incoming = Provider {
            id: existing.id.clone(),
            name: existing.name.clone(),
            settings_config: serde_json::json!({
                "env": { "ANTHROPIC_BASE_URL": "https://team.example.com/v1" }
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
        assert!(should_report_conflict(&existing, &incoming, "claude"));
    }
}
