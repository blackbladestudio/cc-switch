//! 使用统计相关命令

use crate::error::AppError;
use crate::services::usage_stats::*;
use crate::store::AppState;
use rust_decimal::Decimal;
use std::str::FromStr;
use tauri::State;

/// 获取使用量汇总
#[tauri::command]
pub fn get_usage_summary(
    state: State<'_, AppState>,
    start_date: Option<i64>,
    end_date: Option<i64>,
    app_type: Option<String>,
    provider_name: Option<String>,
    model: Option<String>,
) -> Result<UsageSummary, AppError> {
    state.db.get_usage_summary(
        start_date,
        end_date,
        app_type.as_deref(),
        provider_name.as_deref(),
        model.as_deref(),
    )
}

/// 获取按 app_type 拆分的使用量汇总
#[tauri::command]
pub fn get_usage_summary_by_app(
    state: State<'_, AppState>,
    start_date: Option<i64>,
    end_date: Option<i64>,
    provider_name: Option<String>,
    model: Option<String>,
) -> Result<Vec<UsageSummaryByApp>, AppError> {
    state.db.get_usage_summary_by_app(
        start_date,
        end_date,
        provider_name.as_deref(),
        model.as_deref(),
    )
}

/// 获取每日趋势
#[tauri::command]
pub fn get_usage_trends(
    state: State<'_, AppState>,
    start_date: Option<i64>,
    end_date: Option<i64>,
    app_type: Option<String>,
    provider_name: Option<String>,
    model: Option<String>,
) -> Result<Vec<DailyStats>, AppError> {
    state.db.get_daily_trends(
        start_date,
        end_date,
        app_type.as_deref(),
        provider_name.as_deref(),
        model.as_deref(),
    )
}

/// 获取 Provider 统计
#[tauri::command]
pub fn get_provider_stats(
    state: State<'_, AppState>,
    start_date: Option<i64>,
    end_date: Option<i64>,
    app_type: Option<String>,
    provider_name: Option<String>,
    model: Option<String>,
) -> Result<Vec<ProviderStats>, AppError> {
    state.db.get_provider_stats(
        start_date,
        end_date,
        app_type.as_deref(),
        provider_name.as_deref(),
        model.as_deref(),
    )
}

/// 获取模型统计
#[tauri::command]
pub fn get_model_stats(
    state: State<'_, AppState>,
    start_date: Option<i64>,
    end_date: Option<i64>,
    app_type: Option<String>,
    provider_name: Option<String>,
    model: Option<String>,
) -> Result<Vec<ModelStats>, AppError> {
    state.db.get_model_stats(
        start_date,
        end_date,
        app_type.as_deref(),
        provider_name.as_deref(),
        model.as_deref(),
    )
}

/// 获取请求日志列表
#[tauri::command]
pub fn get_request_logs(
    state: State<'_, AppState>,
    filters: LogFilters,
    page: u32,
    page_size: u32,
) -> Result<PaginatedLogs, AppError> {
    state.db.get_request_logs(&filters, page, page_size)
}

/// 获取单个请求详情
#[tauri::command]
pub fn get_request_detail(
    state: State<'_, AppState>,
    request_id: String,
) -> Result<Option<RequestLogDetail>, AppError> {
    state.db.get_request_detail(&request_id)
}

/// 获取内置定价覆盖层的 CNY→内部单位(USD)汇率。
/// 前端用于把后端返回的 USD 成本数值 × rate 换算成 CNY 显示。
#[tauri::command]
pub fn get_pricing_rate(state: State<'_, AppState>) -> Result<String, AppError> {
    let overlay = state
        .user_pricing
        .read()
        .map_err(|e| AppError::Database(format!("读取内置定价覆盖层失败: {e}")))?;
    Ok(overlay.rate.to_string())
}

/// 获取模型定价列表
#[tauri::command]
pub fn get_model_pricing(state: State<'_, AppState>) -> Result<Vec<ModelPricingInfo>, AppError> {
    log::info!("获取模型定价列表");
    state.db.ensure_model_pricing_seeded()?;

    let db = state.db.clone();
    let conn = crate::database::lock_conn!(db.conn);

    // 检查表是否存在
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='model_pricing'",
            [],
            |row| row.get::<_, i64>(0).map(|count| count > 0),
        )
        .unwrap_or(false);

    if !table_exists {
        log::error!("model_pricing 表不存在,可能需要重启应用以触发数据库迁移");
        return Ok(Vec::new());
    }

    let mut stmt = conn.prepare(
        "SELECT model_id, display_name, input_cost_per_million, output_cost_per_million,
                cache_read_cost_per_million, cache_creation_cost_per_million
         FROM model_pricing
         ORDER BY display_name",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ModelPricingInfo {
            model_id: row.get(0)?,
            display_name: row.get(1)?,
            input_cost_per_million: row.get(2)?,
            output_cost_per_million: row.get(3)?,
            cache_read_cost_per_million: row.get(4)?,
            cache_creation_cost_per_million: row.get(5)?,
            source: "builtin".to_string(),
        })
    })?;

    let mut pricing = Vec::new();
    for row in rows {
        pricing.push(row?);
    }

    // 合并内置定价覆盖层（编译期嵌入资源）：覆盖同 model_id 的内置行，追加新增行。
    // 覆盖层 CNY 价 ÷ rate 折成内部单位，与内置行口径一致。
    // 匹配口径与计费路径（logger.get_model_pricing → overlay.lookup）保持一致：
    // 用 model_pricing_candidates 生成候选名逐个比对，而非裸 model_id 精确匹配——
    // 否则 DB 里 gpt-5.5-2025-08-07 这类带后缀的行不会被覆盖层 gpt-5.5 覆盖，
    // 但计费时却能命中，导致 UI 显示价与实际计费价不一致。
    let overlay = state
        .user_pricing
        .read()
        .map_err(|e| AppError::Database(format!("读取用户定价覆盖层失败: {e}")))?;
    if !overlay.is_empty() {
        let rate = if overlay.rate.is_zero() {
            Decimal::ONE
        } else {
            overlay.rate
        };
        let to_internal = |v: Decimal| (v / rate).to_string();
        let mut consumed_overlay_keys: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        // 先按候选名把内置行替换成覆盖层价（命中即替换，与计费路径一致）
        for info in pricing.iter_mut() {
            let candidates = model_pricing_candidates(&info.model_id);
            let hit = candidates
                .iter()
                .find_map(|c| overlay.models.get(c).map(|e| (c.clone(), e)));
            if let Some((ov_key, entry)) = hit {
                // 命中覆盖层：显示名沿用覆盖层，model_id 保持原 DB 行（用户看到的是
                // 自己库里那条记录被内置价接管），source 标 app。
                info.display_name = entry.display_name.clone();
                info.input_cost_per_million = to_internal(entry.input);
                info.output_cost_per_million = to_internal(entry.output);
                info.cache_read_cost_per_million = to_internal(entry.cache_read);
                info.cache_creation_cost_per_million = to_internal(entry.cache_creation);
                info.source = "app".to_string();
                // 记下已处理的覆盖层 key，避免下面重复追加
                consumed_overlay_keys.insert(ov_key);
            }
        }

        // 追加 DB 里没有、覆盖层独有的模型（如 glm-5.2）
        for (model_id, entry) in &overlay.models {
            if consumed_overlay_keys.contains(model_id) {
                continue;
            }
            pricing.push(ModelPricingInfo {
                model_id: model_id.clone(),
                display_name: entry.display_name.clone(),
                input_cost_per_million: to_internal(entry.input),
                output_cost_per_million: to_internal(entry.output),
                cache_read_cost_per_million: to_internal(entry.cache_read),
                cache_creation_cost_per_million: to_internal(entry.cache_creation),
                source: "app".to_string(),
            });
        }
        pricing.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    }

    log::info!("成功获取 {} 条模型定价数据", pricing.len());
    Ok(pricing)
}

/// 更新模型定价
#[tauri::command]
pub fn update_model_pricing(
    state: State<'_, AppState>,
    model_id: String,
    display_name: String,
    input_cost: String,
    output_cost: String,
    cache_read_cost: String,
    cache_creation_cost: String,
) -> Result<(), AppError> {
    let db = state.db.clone();
    let model_id = model_id.trim().to_string();
    let display_name = display_name.trim().to_string();
    if model_id.is_empty() {
        return Err(AppError::localized(
            "usage.modelIdRequired",
            "模型 ID 不能为空",
            "Model ID is required",
        ));
    }
    if display_name.is_empty() {
        return Err(AppError::localized(
            "usage.displayNameRequired",
            "显示名称不能为空",
            "Display name is required",
        ));
    }

    // 内置定价覆盖层行只读（随应用发布），禁止在 UI 内修改。
    // 用候选名匹配，与计费/合并口径一致：带后缀的变体若被覆盖层接管同样禁改。
    {
        let overlay = state
            .user_pricing
            .read()
            .map_err(|e| AppError::Database(format!("读取内置定价覆盖层失败: {e}")))?;
        if overlay.contains_model(&model_id) {
            return Err(AppError::localized(
                "usage.userPricingReadOnly",
                format!("该模型定价为应用内置，不可在 UI 修改: {model_id}"),
                format!("This model's pricing is built into the app and cannot be edited here: {model_id}"),
            ));
        }
    }

    for (label, value) in [
        ("input_cost", &input_cost),
        ("output_cost", &output_cost),
        ("cache_read_cost", &cache_read_cost),
        ("cache_creation_cost", &cache_creation_cost),
    ] {
        let parsed = Decimal::from_str(value.trim()).map_err(|e| {
            AppError::localized(
                "usage.invalidPrice",
                format!("{label} 价格无效: {value} - {e}"),
                format!("{label} price is invalid: {value} - {e}"),
            )
        })?;
        if parsed < Decimal::ZERO {
            return Err(AppError::localized(
                "usage.invalidPrice",
                format!("{label} 价格必须为非负数: {value}"),
                format!("{label} price must be non-negative: {value}"),
            ));
        }
    }

    {
        let conn = crate::database::lock_conn!(db.conn);
        conn.execute(
            "INSERT OR REPLACE INTO model_pricing (
                model_id, display_name, input_cost_per_million, output_cost_per_million,
                cache_read_cost_per_million, cache_creation_cost_per_million
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                model_id,
                display_name,
                input_cost.trim(),
                output_cost.trim(),
                cache_read_cost.trim(),
                cache_creation_cost.trim()
            ],
        )
        .map_err(|e| AppError::Database(format!("更新模型定价失败: {e}")))?;
    }

    // 历史用量回填需用同一覆盖层重算（内置定价缺失模型的历史 0 成本行
    // 也能补回来）。先取出覆盖层快照，避免持有锁跨 DB 写入。
    let overlay_snapshot = state.user_pricing.read().map(|guard| guard.clone()).ok();

    if let Err(e) = db.backfill_missing_usage_costs_for_model(&model_id, overlay_snapshot.as_ref())
    {
        log::warn!("模型定价更新后回填历史用量成本失败 (model_id={model_id}): {e}");
    }

    Ok(())
}

/// 检查 Provider 使用限额
#[tauri::command]
pub fn check_provider_limits(
    state: State<'_, AppState>,
    provider_id: String,
    app_type: String,
) -> Result<crate::services::usage_stats::ProviderLimitStatus, AppError> {
    state.db.check_provider_limits(&provider_id, &app_type)
}

/// 删除模型定价
#[tauri::command]
pub fn delete_model_pricing(state: State<'_, AppState>, model_id: String) -> Result<(), AppError> {
    // 内置定价覆盖层行只读（随应用发布），禁止在 UI 内删除
    {
        let overlay = state
            .user_pricing
            .read()
            .map_err(|e| AppError::Database(format!("读取内置定价覆盖层失败: {e}")))?;
        if overlay.contains_model(&model_id) {
            return Err(AppError::localized(
                "usage.userPricingReadOnly",
                format!("该模型定价为应用内置，不可删除: {model_id}"),
                format!(
                    "This model's pricing is built into the app and cannot be deleted: {model_id}"
                ),
            ));
        }
    }

    let db = state.db.clone();
    let conn = crate::database::lock_conn!(db.conn);

    conn.execute(
        "DELETE FROM model_pricing WHERE model_id = ?1",
        rusqlite::params![model_id],
    )
    .map_err(|e| AppError::Database(format!("删除模型定价失败: {e}")))?;

    log::info!("已删除模型定价: {model_id}");
    Ok(())
}

/// 手动触发会话日志同步
#[tauri::command]
pub async fn sync_session_usage(
    state: State<'_, AppState>,
) -> Result<crate::services::session_usage::SessionSyncResult, AppError> {
    let db = state.db.clone();
    let overlay_snapshot = state.user_pricing.read().ok().map(|g| g.clone());
    let _guard = crate::services::session_usage::session_sync_mutex()
        .lock()
        .await;
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::session_usage::sync_all_unlocked(&db, overlay_snapshot.as_ref())
    })
    .await
    .map_err(|error| AppError::Message(format!("会话用量同步任务失败: {error}")))
}

/// Codex reset 成功后，无论重导是否导入新行或返回错误，都必须通知前端刷新。
/// 调用方应只在 reset 成功后调用，避免把未发生的数据变更误报为重建完成。
fn finish_codex_rebuild(
    result: Result<crate::services::session_usage::SessionSyncResult, AppError>,
) -> Result<crate::services::session_usage::SessionSyncResult, AppError> {
    crate::usage_events::notify_log_recorded();
    result
}

/// 备份数据库后，仅重建 Codex session 用量。锁覆盖 backup → reset → import
/// 整个序列，避免后台同步在清理和重导之间插入数据。
#[tauri::command]
pub async fn rebuild_codex_usage(
    state: State<'_, AppState>,
) -> Result<crate::services::session_usage::SessionSyncResult, AppError> {
    let db = state.db.clone();
    let _guard = crate::services::session_usage::session_sync_mutex()
        .lock()
        .await;
    tauri::async_runtime::spawn_blocking(move || {
        db.backup_database_file()?;
        db.reset_codex_usage()?;
        let result = crate::services::session_usage_codex::sync_codex_usage(&db);
        finish_codex_rebuild(result)
    })
    .await
    .map_err(|error| AppError::Message(format!("Codex 用量重建任务失败: {error}")))?
}

/// 获取数据来源分布
#[tauri::command]
pub fn get_usage_data_sources(
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::session_usage::DataSourceSummary>, AppError> {
    crate::services::session_usage::get_data_source_breakdown(&state.db)
}

/// 模型定价信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPricingInfo {
    pub model_id: String,
    pub display_name: String,
    pub input_cost_per_million: String,
    pub output_cost_per_million: String,
    pub cache_read_cost_per_million: String,
    pub cache_creation_cost_per_million: String,
    /// 定价来源："builtin"(DB 内置表，可在 UI 编辑) | "app"(应用内置资源，只读)
    #[serde(default = "default_pricing_source")]
    pub source: String,
}

fn default_pricing_source() -> String {
    "builtin".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_rebuild_notifies_when_reimport_is_empty() {
        crate::usage_events::take_test_notify_count();

        let result = finish_codex_rebuild(Ok(
            crate::services::session_usage::SessionSyncResult::default(),
        ))
        .expect("空重导应成功");

        assert_eq!(result.imported, 0);
        assert_eq!(crate::usage_events::take_test_notify_count(), 1);
    }

    #[test]
    fn codex_rebuild_notifies_when_reimport_fails_after_reset() {
        crate::usage_events::take_test_notify_count();

        let result = finish_codex_rebuild(Err(AppError::Message(
            "synthetic reimport failure".to_string(),
        )));

        assert!(result.is_err());
        assert_eq!(crate::usage_events::take_test_notify_count(), 1);
    }
}
