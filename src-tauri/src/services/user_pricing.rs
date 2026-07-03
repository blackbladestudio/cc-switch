//! 内置模型定价表（随应用发布）
//!
//! 定价以人民币（CNY / 1M tokens）填写，编译期通过 `include_str!` 嵌入二进制
//! （`src/resources/builtin_pricing.json`），启动加载到内存覆盖层。查找时优先
//! 命中本表，miss 再回退 DB `model_pricing` 表。
//!
//! 计算器 `ModelPricing` 的字段是无单位 `Decimal`，DB 种子价已按
//! `CNY ÷ rate` 折成内部单位（见 `database::schema` 注释）。这里在加载时把
//! CNY 价 ÷ `rate` 折成同一内部单位，保证历史成本数据口径一致。升级应用即
//! 更新本表，用户无需（也无法）手动编辑。

use std::collections::HashMap;

use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde::Deserialize;

use crate::proxy::usage::calculator::ModelPricing;
use crate::services::usage_stats::{is_placeholder_pricing_model, model_pricing_candidates};

/// 默认 CNY→内部单位汇率（与种子价 historical 折算一致）
const DEFAULT_RATE: &str = "7.14";

/// 内置定价资源（编译期嵌入）
const BUILTIN_PRICING_JSON: &str = include_str!("../resources/builtin_pricing.json");

/// 单条用户定价（CNY / 1M tokens）。仅内存使用，不序列化回磁盘——
/// 唯一来源是编译期嵌入的内置资源（只读）。
/// `rename_all = "camelCase"`：JSON 用 camelCase（displayName/cacheRead/...），
/// struct 字段用 snake_case；不加这行 serde 会按字段名精确匹配，导致
/// cacheRead/cacheCreation 落到默认值 0，所有覆盖层模型的缓存计费被静默清零。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPricingEntry {
    #[serde(default)]
    pub display_name: String,
    pub input: Decimal,
    pub output: Decimal,
    #[serde(default)]
    pub cache_read: Decimal,
    #[serde(default)]
    pub cache_creation: Decimal,
}

/// 反序列化中间结构：JSON 数字友好。
/// `rust_decimal` 默认 serde 只接受字符串，不接受 JSON 数字；这里用
/// `serde_json::Number` 接住数字，再经 `to_string()` → `Decimal::from_str`
/// 转换，避免 f64 中转带来的浮点尾迹（如 0.1 无法精确表示）。
/// 加载后丢弃，转成 UserPricingConfig（Decimal）。
#[derive(Debug, Deserialize)]
struct RawUserPricingConfig {
    #[serde(default = "default_rate_raw")]
    rate: serde_json::Number,
    #[serde(default)]
    models: HashMap<String, RawUserPricingEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUserPricingEntry {
    #[serde(default)]
    display_name: String,
    input: serde_json::Number,
    output: serde_json::Number,
    #[serde(default = "default_zero_number")]
    cache_read: serde_json::Number,
    #[serde(default = "default_zero_number")]
    cache_creation: serde_json::Number,
}

fn default_rate_raw() -> serde_json::Number {
    // 7.14 在 JSON 数字范围内可精确表示为 Number（内部走字符串往返）
    serde_json::Number::from_f64(7.14).unwrap_or_else(|| serde_json::Number::from(7))
}

fn default_zero_number() -> serde_json::Number {
    serde_json::Number::from(0)
}

fn number_to_decimal(v: &serde_json::Number) -> Decimal {
    // Number::to_string() 保留 JSON 原始字面量形式（"1.4"、"7.14"），
    // 再交给 Decimal::from_str，全程无 f64 参与，精度无损。
    Decimal::from_str(&v.to_string()).unwrap_or(Decimal::ZERO)
}

/// 用户定价配置文件
#[derive(Debug, Clone, Deserialize)]
pub struct UserPricingConfig {
    pub rate: Decimal,
    #[serde(default)]
    pub models: HashMap<String, UserPricingEntry>,
}

impl Default for UserPricingConfig {
    fn default() -> Self {
        Self {
            rate: default_rate(),
            models: HashMap::new(),
        }
    }
}

fn default_rate() -> Decimal {
    Decimal::from_str(DEFAULT_RATE).unwrap_or(Decimal::ONE)
}

impl UserPricingConfig {
    /// 从编译期嵌入的内置定价资源加载；解析失败时返回空配置（记 warn，不阻断启动）
    pub fn load() -> Self {
        Self::from_json(BUILTIN_PRICING_JSON)
    }

    /// 从 JSON 字符串解析（供测试与 load 复用）
    pub fn from_json(json: &str) -> Self {
        match serde_json::from_str::<RawUserPricingConfig>(json) {
            Ok(raw) => {
                let rate = number_to_decimal(&raw.rate);
                if rate.is_zero() {
                    log::warn!(
                        "[BUILTIN-PRICING] 内置定价 rate 为 0，已忽略（避免除零）"
                    );
                    return Self::default();
                }
                let models = raw
                    .models
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            k,
                            UserPricingEntry {
                                display_name: v.display_name,
                                input: number_to_decimal(&v.input),
                                output: number_to_decimal(&v.output),
                                cache_read: number_to_decimal(&v.cache_read),
                                cache_creation: number_to_decimal(&v.cache_creation),
                            },
                        )
                    })
                    .collect();
                let cfg = UserPricingConfig { rate, models };
                log::info!(
                    "[BUILTIN-PRICING] 已加载 {} 条内置定价，rate={}",
                    cfg.models.len(),
                    cfg.rate
                );
                cfg
            }
            Err(e) => {
                log::warn!("[BUILTIN-PRICING] 解析内置定价失败，忽略: {e}");
                Self::default()
            }
        }
    }

    /// 按模型名查找定价。复用 `model_pricing_candidates` 生成候选名，
    /// 逐个精确匹配，命中后把 CNY 字段 ÷ rate 折成内部单位。
    pub fn lookup(&self, model_id: &str) -> Option<ModelPricing> {
        let candidates = model_pricing_candidates(model_id);
        for candidate in &candidates {
            if is_placeholder_pricing_model(candidate) {
                continue;
            }
            if let Some(entry) = self.models.get(candidate) {
                return self.entry_to_pricing(entry).ok();
            }
        }
        None
    }

    fn entry_to_pricing(&self, e: &UserPricingEntry) -> Result<ModelPricing, rust_decimal::Error> {
        let rate = if self.rate.is_zero() {
            Decimal::ONE
        } else {
            self.rate
        };
        Ok(ModelPricing {
            input_cost_per_million: e.input / rate,
            output_cost_per_million: e.output / rate,
            cache_read_cost_per_million: e.cache_read / rate,
            cache_creation_cost_per_million: e.cache_creation / rate,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    /// 该 model_id 是否会被覆盖层接管计价（候选名命中即 true）。
    /// 用于 UI 只读守卫：与计费/合并路径同一口径，避免按裸 model_id 精确匹配
    /// 漏掉带后缀的变体（如 gpt-5.5-2025-08-07 被 gpt-5.5 接管）。
    pub fn contains_model(&self, model_id: &str) -> bool {
        let trimmed = model_id.trim();
        if trimmed.is_empty() {
            return false;
        }
        for candidate in model_pricing_candidates(trimmed) {
            if is_placeholder_pricing_model(&candidate) {
                continue;
            }
            if self.models.contains_key(&candidate) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn cfg(rate: &str, models: &[(&str, &str, &str)]) -> UserPricingConfig {
        let mut m = HashMap::new();
        for (id, input, output) in models {
            m.insert(
                id.to_string(),
                UserPricingEntry {
                    display_name: id.to_string(),
                    input: Decimal::from_str(input).unwrap(),
                    output: Decimal::from_str(output).unwrap(),
                    cache_read: Decimal::ZERO,
                    cache_creation: Decimal::ZERO,
                },
            );
        }
        UserPricingConfig {
            rate: Decimal::from_str(rate).unwrap(),
            models: m,
        }
    }

    #[test]
    fn lookup_exact_match_and_rate_conversion() {
        let c = cfg("7.14", &[("glm-5.2", "1.4", "4.4")]);
        let p = c.lookup("glm-5.2").expect("应命中 glm-5.2");
        // 1.4 CNY ÷ 7.14 ≈ 0.1961
        assert_eq!(p.input_cost_per_million, Decimal::from_str("1.4").unwrap() / Decimal::from_str("7.14").unwrap());
    }

    #[test]
    fn lookup_miss_returns_none() {
        let c = cfg("7.14", &[("glm-5.2", "1.4", "4.4")]);
        assert!(c.lookup("glm-9").is_none());
    }

    #[test]
    fn lookup_candidate_normalization_matches() {
        // 候选生成会去掉命名空间前缀；providers/moonshot/glm-5.2 应命中 glm-5.2
        let c = cfg("7.14", &[("glm-5.2", "1.4", "4.4")]);
        let p = c.lookup("moonshot.glm-5.2").expect("去前缀后应命中");
        assert!(p.input_cost_per_million > Decimal::ZERO);
    }

    #[test]
    fn placeholder_model_returns_none() {
        let c = cfg("7.14", &[("glm-5.2", "1.4", "4.4")]);
        assert!(c.lookup("").is_none());
        assert!(c.lookup("unknown").is_none());
    }

    #[test]
    fn from_json_invalid_returns_empty() {
        let c = UserPricingConfig::from_json("not json");
        assert!(c.is_empty());
    }

    #[test]
    fn from_json_valid() {
        let content = serde_json::json!({
            "rate": 7.0_f64,
            "models": {
                "glm-5.2": { "input": 1.4_f64, "output": 4.4_f64 }
            }
        });
        let c = UserPricingConfig::from_json(&content.to_string());
        assert_eq!(c.rate, Decimal::from(7));
        assert!(c.lookup("glm-5.2").is_some());
    }

    #[test]
    fn load_builtin_resource_has_glm52() {
        // 编译期嵌入的内置定价资源必须能解析，且包含 glm-5.2
        let c = UserPricingConfig::load();
        assert!(!c.is_empty(), "内置定价表不应为空");
        assert!(c.lookup("glm-5.2").is_some(), "内置定价应含 glm-5.2");
    }

    #[test]
    fn camel_case_fields_are_deserialized() {
        // 回归：JSON 用 camelCase（cacheRead/displayName），struct 用 snake_case。
        // 没有 rename_all="camelCase" 时 cacheRead 会被丢弃、cache_read 默认 0，
        // 所有覆盖层模型的缓存计费被静默清零——这正是 glm-5.2 cache_read_cost=0
        // 的根因。这里钉死 camelCase 解析，防止回退。
        let c = UserPricingConfig::load();
        let glm = c.models.get("glm-5.2").expect("glm-5.2 必须存在");
        assert_eq!(
            glm.cache_read,
            Decimal::from_str("1.6").unwrap(),
            "cacheRead 必须从 JSON 读到 1.6，而非默认 0"
        );
        assert_eq!(
            glm.display_name, "GLM-5.2",
            "displayName 必须从 JSON 读到，而非默认空串"
        );
        // 折算后 cache_read_cost_per_million = 1.6 / 7.14 ≈ 0.2241，不应为 0
        let p = c.lookup("glm-5.2").expect("应命中");
        assert!(
            p.cache_read_cost_per_million > Decimal::ZERO,
            "cache_read 折算后不应为 0"
        );
    }
}
