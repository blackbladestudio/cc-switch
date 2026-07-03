use crate::database::Database;
use crate::services::user_pricing::UserPricingConfig;
use crate::services::{ProxyService, UsageCache};
use std::sync::{Arc, RwLock};

/// 全局应用状态
pub struct AppState {
    pub db: Arc<Database>,
    pub proxy_service: ProxyService,
    pub usage_cache: Arc<UsageCache>,
    /// 内置模型定价覆盖层（CNY，编译期嵌入资源加载）
    pub user_pricing: Arc<RwLock<UserPricingConfig>>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(db: Arc<Database>) -> Self {
        let user_pricing = Arc::new(RwLock::new(UserPricingConfig::load()));
        let proxy_service = ProxyService::new_with_overlay(db.clone(), user_pricing.clone());

        Self {
            db,
            proxy_service,
            usage_cache: Arc::new(UsageCache::new()),
            user_pricing,
        }
    }
}
