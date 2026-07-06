//! 工作室 Studio Admin 统一登录——自动获取模型 apiKey。
//!
//! 流程（参考 studio_admin/examples/local-app-login.ts）：
//! 1. 前端调 `auth_studio_login_start(state)` → Rust 绑随机端口 `127.0.0.1:<port>` 起一次性 HTTP server，
//!    返回登录 URL `{ADMIN_URL}/login?redirect=http://127.0.0.1:<port>/callback&state=<state>`。
//! 2. 前端 `open_external` 打开浏览器，用户在 admin 完成飞书登录。
//! 3. admin 跳回本地 server `?code=<一次性code>&state=<state>`。本地立即响应浏览器「登录成功」。
//! 4. Rust 用 code POST `{ADMIN_URL}/api/auth/token` 换 JWT token（7 天有效）。
//! 5. Rust 用 token 调 `{ADMIN_URL}/api/me/api-keys` 取 key 列表（本应用只分配一个专用 key），
//!    再调 `{ADMIN_URL}/api/me/api-keys/<id>/reveal` 拿明文 apiKey。
//! 6. emit `studio-auth-callback` 事件给前端，payload `{ apiKey, accountId, keyId, token }`。
//!    前端把 apiKey 写进 provider 字段，token/keyId/accountId 调 `auth_studio_save_account` 落盘。
//!
//! 启动静默刷新：用缓存的 token 调 reveal 接口拿最新 key 明文写回 provider；token 401 则标 needsRelogin。
//!
//! 与 Copilot/Codex 的区别：拿回来的 apiKey 是长期模型 key，直接写进 provider 字段，
//! 不走 forwarder 注入；token 仅用于启动时静默 reveal key 校正（admin 可能改了 key）。

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tokio::sync::RwLock;

/// Studio Admin 通过 gateway 暴露的地址（写死常量）。
const ADMIN_URL: &str = "http://inner.blackblade.com";

#[derive(Debug, thiserror::Error)]
pub enum StudioAuthError {
    #[error("账号未找到: {0}")]
    AccountNotFound(String),
    #[error("token 失效，需重新登录")]
    NeedsRelogin,
    #[error("admin 返回错误: {0}")]
    Admin(String),
    #[error("响应缺少必要字段: {0}")]
    MissingField(&'static str),
    #[error("HTTP 请求失败: {0}")]
    Http(#[from] reqwest::Error),
    #[error("解析响应失败: {0}")]
    Parse(String),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioAccountData {
    pub account_id: String,
    pub key_id: String,
    pub token: String,
    /// 用户显示名（来自 /api/auth/me 的 data.name），仅用于 UI 展示
    #[serde(default)]
    pub account_name: Option<String>,
    /// 最近一次成功 reveal 的时间戳（秒）
    #[serde(default)]
    pub last_refreshed_at: Option<i64>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct StudioAuthStore {
    version: u32,
    /// key = accountId
    accounts: HashMap<String, StudioAccountData>,
}

/// 工作室账号认证管理器
pub struct StudioAuthManager {
    store: RwLock<StudioAuthStore>,
    storage_path: PathBuf,
    http: Client,
}

/// 登录回调成功后 emit 给前端的事件 payload
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioLoginResult {
    pub api_key: String,
    pub account_id: String,
    pub account_name: Option<String>,
    pub key_id: String,
    pub token: String,
}

impl StudioAuthManager {
    pub fn new(data_dir: PathBuf) -> Self {
        let storage_path = data_dir.join("studio_auth.json");
        let mgr = Self {
            store: RwLock::new(StudioAuthStore::default()),
            storage_path,
            http: Client::new(),
        };
        if let Err(e) = mgr.load_from_disk() {
            log::warn!("[StudioAuth] 加载存储失败: {e}");
        }
        mgr
    }

    /// 拼登录 URL：`{ADMIN_URL}/login?redirect=<本地callback>&state=<state>`
    /// `local_callback` 形如 `http://127.0.0.1:<port>/callback`。
    pub fn build_login_url(local_callback: &str, state: &str) -> String {
        format!(
            "{ADMIN_URL}/login?redirect={}&state={state}",
            percent_encode_query(local_callback)
        )
    }

    /// 用一次性 code 换 JWT token。
    /// code 30 秒过期、仅可使用一次。
    pub async fn exchange_code_for_token(&self, code: &str) -> Result<String, StudioAuthError> {
        let resp = self
            .http
            .post(format!("{ADMIN_URL}/api/auth/token"))
            .json(&serde_json::json!({ "code": code }))
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;
        if body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            let msg = body
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("换取 token 失败");
            return Err(StudioAuthError::Admin(msg.to_string()));
        }
        let token = body
            .get("data")
            .and_then(|d| d.get("token"))
            .and_then(|t| t.as_str())
            .ok_or(StudioAuthError::MissingField("token"))?;
        Ok(token.to_string())
    }

    /// 用 token 调 `/api/auth/me` 拿当前用户 accountId + 显示名。
    /// 401 → NeedsRelogin。
    pub async fn fetch_account(&self, token: &str) -> Result<(String, Option<String>), StudioAuthError> {
        let resp = self
            .http
            .get(format!("{ADMIN_URL}/api/auth/me"))
            .bearer_auth(token)
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(StudioAuthError::NeedsRelogin);
        }
        let body: serde_json::Value = resp.json().await?;
        // 取 data.id（或 data.user.id，兼容两种形状）；data.name 为显示名（可空）
        let data = body
            .get("data")
            .ok_or(StudioAuthError::MissingField("data"))?;
        let id = data
            .get("id")
            .or_else(|| data.get("user").and_then(|u| u.get("id")))
            .and_then(|v| v.as_str())
            .ok_or(StudioAuthError::MissingField("data.id"))?;
        let name = data
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        Ok((id.to_string(), name))
    }

    /// 用 token 取 key 列表，返回第一个 key 的 id（本应用只分配一个专用 key）。
    pub async fn fetch_first_key_id(&self, token: &str) -> Result<String, StudioAuthError> {
        let resp = self
            .http
            .get(format!("{ADMIN_URL}/api/me/api-keys"))
            .bearer_auth(token)
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(StudioAuthError::NeedsRelogin);
        }
        let body: serde_json::Value = resp.json().await?;
        let key_id = body
            .get("data")
            .and_then(|d| d.get("keys"))
            .and_then(|k| k.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("id"))
            .and_then(|v| v.as_str())
            .ok_or(StudioAuthError::MissingField("data.keys[0].id"))?;
        Ok(key_id.to_string())
    }

    /// 用 token reveal 某个 key 的明文。
    /// 401 → NeedsRelogin。
    pub async fn reveal_api_key(
        &self,
        token: &str,
        key_id: &str,
    ) -> Result<String, StudioAuthError> {
        let resp = self
            .http
            .get(format!("{ADMIN_URL}/api/me/api-keys/{key_id}/reveal"))
            .bearer_auth(token)
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(StudioAuthError::NeedsRelogin);
        }
        let body: serde_json::Value = resp.json().await?;
        let key = body
            .get("data")
            .and_then(|d| d.get("key"))
            .and_then(|v| v.as_str())
            .ok_or(StudioAuthError::MissingField("data.key"))?;
        Ok(key.to_string())
    }

    /// 完整登录收尾：code → token → accountId → keyId → apiKey。
    /// 由本地 HTTP callback handler 在拿到 code 后调用。
    pub async fn complete_login(&self, code: &str) -> Result<StudioLoginResult, StudioAuthError> {
        let token = self.exchange_code_for_token(code).await?;
        let (account_id, account_name) = self.fetch_account(&token).await?;
        let key_id = self.fetch_first_key_id(&token).await?;
        let api_key = self.reveal_api_key(&token, &key_id).await?;
        Ok(StudioLoginResult {
            api_key,
            account_id,
            account_name,
            key_id,
            token,
        })
    }

    /// 登录成功后落盘账号（token + keyId + accountId + 显示名）。
    pub async fn save_account(
        &self,
        account_id: &str,
        key_id: &str,
        token: &str,
        account_name: Option<&str>,
    ) {
        let mut store = self.store.write().await;
        store.accounts.insert(
            account_id.to_string(),
            StudioAccountData {
                account_id: account_id.to_string(),
                key_id: key_id.to_string(),
                token: token.to_string(),
                account_name: account_name.map(|s| s.to_string()),
                last_refreshed_at: None,
            },
        );
        if let Err(e) = self.save_to_disk(&store) {
            log::error!("[StudioAuth] 保存账号失败: {e}");
        }
    }

    /// 取账号显示名（供前端状态查询展示）。
    pub async fn get_account_name(&self, account_id: &str) -> Option<String> {
        self.store
            .read()
            .await
            .accounts
            .get(account_id)
            .and_then(|acc| acc.account_name.clone())
    }

    /// 启动静默刷新：用缓存的 token 调 reveal 接口拿最新 key 明文。
    /// 401 / token 失效 → `NeedsRelogin`，调用方据此设 `meta.authBinding.needsRelogin = true`。
    pub async fn refresh_api_key(&self, account_id: &str) -> Result<String, StudioAuthError> {
        let (token, key_id) = {
            let store = self.store.read().await;
            let acc = store
                .accounts
                .get(account_id)
                .ok_or_else(|| StudioAuthError::AccountNotFound(account_id.to_string()))?;
            (acc.token.clone(), acc.key_id.clone())
        };

        let api_key = self.reveal_api_key(&token, &key_id).await?;

        let now = chrono::Utc::now().timestamp();
        let mut store = self.store.write().await;
        if let Some(acc) = store.accounts.get_mut(account_id) {
            acc.last_refreshed_at = Some(now);
        }
        if let Err(e) = self.save_to_disk(&store) {
            log::warn!("[StudioAuth] 更新 last_refreshed_at 落盘失败: {e}");
        }

        Ok(api_key)
    }

    /// 该账号是否已登录（本地有 token 缓存）
    pub async fn has_account(&self, account_id: &str) -> bool {
        self.store.read().await.accounts.contains_key(account_id)
    }

    pub async fn list_account_ids(&self) -> Vec<String> {
        self.store.read().await.accounts.keys().cloned().collect()
    }

    /// 列出所有已登录账号的 `(account_id, account_name)`，供认证中心展示。
    pub async fn list_accounts(&self) -> Vec<(String, Option<String>)> {
        self.store
            .read()
            .await
            .accounts
            .iter()
            .map(|(id, acc)| (id.clone(), acc.account_name.clone()))
            .collect()
    }

    /// 移除账号缓存（用户登出 / 切回手动模式时调）
    pub async fn remove_account(&self, account_id: &str) {
        let mut store = self.store.write().await;
        if store.accounts.remove(account_id).is_some() {
            if let Err(e) = self.save_to_disk(&store) {
                log::error!("[StudioAuth] 移除账号落盘失败: {e}");
            }
        }
    }

    fn load_from_disk(&self) -> Result<(), StudioAuthError> {
        if !self.storage_path.exists() {
            return Ok(());
        }
        let content = fs::read_to_string(&self.storage_path)?;
        let store: StudioAuthStore = serde_json::from_str(&content)?;
        if let Ok(mut guard) = self.store.try_write() {
            *guard = store;
        }
        Ok(())
    }

    fn save_to_disk(&self, store: &StudioAuthStore) -> Result<(), StudioAuthError> {
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(store)?;

        let parent = self
            .storage_path
            .parent()
            .ok_or_else(|| StudioAuthError::Parse("无效的存储路径".to_string()))?;
        let file_name = self
            .storage_path
            .file_name()
            .ok_or_else(|| StudioAuthError::Parse("无效的存储文件名".to_string()))?
            .to_string_lossy()
            .to_string();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let tmp_path = parent.join(format!("{file_name}.tmp.{ts}"));

        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&tmp_path)?;
            file.write_all(content.as_bytes())?;
            file.flush()?;
            fs::rename(&tmp_path, &self.storage_path)?;
        }

        #[cfg(windows)]
        {
            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&tmp_path)?;
            file.write_all(content.as_bytes())?;
            file.flush()?;
            if self.storage_path.exists() {
                let _ = fs::remove_file(&self.storage_path);
            }
            fs::rename(&tmp_path, &self.storage_path)?;
        }

        Ok(())
    }
}

/// 该 provider 是否绑定工作室账号自动获取。
pub fn is_studio_provider(meta: &crate::provider::ProviderMeta) -> bool {
    meta.auth_binding
        .as_ref()
        .map(|b| b.auth_provider.as_deref() == Some("studio_account"))
        .unwrap_or(false)
}

/// 把刷新拿到的新 apiKey 写进 provider 的 `settings_config.env[apiKeyField]`。
/// `api_key_field` 为 None 时默认 `ANTHROPIC_AUTH_TOKEN`。
pub fn write_api_key_into_provider(
    provider: &mut crate::provider::Provider,
    api_key_field: Option<&str>,
    new_key: &str,
) {
    let field = api_key_field.unwrap_or("ANTHROPIC_AUTH_TOKEN");
    let root = provider
        .settings_config
        .as_object_mut()
        .expect("settings_config 应为对象");
    let env = root
        .entry("env".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if let Some(env_obj) = env.as_object_mut() {
        env_obj.insert(field.to_string(), serde_json::json!(new_key));
    }
}

/// 标记 provider 的 authBinding 为「需要重新登录」。
pub fn mark_needs_relogin(provider: &mut crate::provider::Provider, needs: bool) {
    if let Some(meta) = provider.meta.as_mut() {
        if let Some(binding) = meta.auth_binding.as_mut() {
            binding.needs_relogin = Some(needs);
        }
    }
}

/// 极简 query 值 percent-编码：编码 `:` `/` `?` `&` `=` `#` ` ` 等不安全字符。
fn percent_encode_query(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let safe = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~');
        if safe {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// 从 HTTP request line 解析 `GET /callback?code=...&state=... HTTP/1.1` 的 query 参数。
fn parse_callback_query(request_line: &str) -> Option<(Option<String>, Option<String>)> {
    // 形如 "GET /callback?code=xxx&state=yyy HTTP/1.1"
    let path = request_line.split_whitespace().nth(1)?;
    let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
    let mut code: Option<String> = None;
    let mut state: Option<String> = None;
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            match k {
                "code" => code = Some(percent_decode(v)),
                "state" => state = Some(percent_decode(v)),
                _ => {}
            }
        }
    }
    Some((code, state))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""),
                16,
            ) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// 绑本地随机端口起一次性 HTTP server，返回登录 URL。
///
/// server 只接收一次连接：admin 登录完成后 redirect 到 `http://127.0.0.1:<port>/callback?code=...&state=...`，
/// Rust 立即响应浏览器「登录成功」，然后用 code 换 token + reveal key，emit `studio-auth-callback` 事件给前端。
/// state 不匹配或缺少 code 时 emit error payload。
pub async fn spawn_login_server(
    app: tauri::AppHandle,
    mgr: std::sync::Arc<tokio::sync::RwLock<StudioAuthManager>>,
    state: String,
) -> Result<String, StudioAuthError> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(StudioAuthError::Io)?;
    let port = listener
        .local_addr()
        .map_err(StudioAuthError::Io)?
        .port();
    let local_callback = format!("http://127.0.0.1:{port}/callback");
    let login_url = StudioAuthManager::build_login_url(&local_callback, &state);

    let state_for_task = state.clone();
    tokio::spawn(async move {
        // 只接受一次连接（admin 跳回）。设 3 分钟超时自动退出。
        let accept = tokio::time::timeout(
            std::time::Duration::from_secs(3 * 60),
            listener.accept(),
        )
        .await;

        let (mut socket, _) = match accept {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                log::warn!("[StudioAuth] 接受回调连接失败: {e}");
                let _ = app.emit(
                    "studio-auth-callback",
                    serde_json::json!({
                        "state": state_for_task,
                        "error": format!("接受连接失败: {e}"),
                    }),
                );
                return;
            }
            Err(_) => {
                log::warn!("[StudioAuth] 登录超时（3 分钟未完成）");
                let _ = app.emit(
                    "studio-auth-callback",
                    serde_json::json!({
                        "state": state_for_task,
                        "error": "登录超时",
                    }),
                );
                return;
            }
        };

        // 读 HTTP request（只取第一行 request line 即可）
        let mut buf = [0u8; 2048];
        let n = match socket.read(&mut buf).await {
            Ok(n) => n,
            Err(e) => {
                log::warn!("[StudioAuth] 读回调请求失败: {e}");
                let _ = app.emit(
                    "studio-auth-callback",
                    serde_json::json!({
                        "state": state_for_task,
                        "error": format!("读请求失败: {e}"),
                    }),
                );
                return;
            }
        };
        let request = String::from_utf8_lossy(&buf[..n]);
        let request_line = request.lines().next().unwrap_or("");
        let (code, cb_state) = parse_callback_query(request_line).unwrap_or_default();

        // 立即响应浏览器
        let body = if code.is_some() {
            "<h1>登录成功，可以关闭此页面</h1>"
        } else {
            "<h1>登录失败：缺少授权码</h1>"
        };
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = socket.write_all(resp.as_bytes()).await;
        let _ = socket.flush().await;

        // state 校验
        if cb_state.as_deref() != Some(state_for_task.as_str()) {
            log::warn!(
                "[StudioAuth] 回调 state 不匹配: expected={state_for_task}, got={cb_state:?}"
            );
            let _ = app.emit(
                "studio-auth-callback",
                serde_json::json!({
                    "state": state_for_task,
                    "error": "state 不匹配",
                }),
            );
            return;
        }

        let code = match code {
            Some(c) => c,
            None => {
                let _ = app.emit(
                    "studio-auth-callback",
                    serde_json::json!({
                        "state": state_for_task,
                        "error": "缺少授权码 code",
                    }),
                );
                return;
            }
        };

        // code → token → accountId → keyId → apiKey
        let mgr_read = mgr.read().await;
        match mgr_read.complete_login(&code).await {
            Ok(result) => {
                // 注入 state，前端按 state 匹配 pending 登录（缺 state 会被丢弃）
                let _ = app.emit(
                    "studio-auth-callback",
                    serde_json::json!({
                        "apiKey": result.api_key,
                        "accountId": result.account_id,
                        "accountName": result.account_name,
                        "keyId": result.key_id,
                        "token": result.token,
                        "state": state_for_task,
                    }),
                );
            }
            Err(e) => {
                log::warn!("[StudioAuth] complete_login 失败: {e}");
                let _ = app.emit(
                    "studio-auth-callback",
                    serde_json::json!({
                        "state": state_for_task,
                        "error": e.to_string(),
                    }),
                );
            }
        }
    });

    Ok(login_url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{
        AuthBinding, AuthBindingSource, Provider, ProviderMeta,
    };

    #[test]
    fn build_login_url_includes_redirect_and_state() {
        let url = StudioAuthManager::build_login_url(
            "http://127.0.0.1:54321/callback",
            "abc-123",
        );
        assert!(url.starts_with(ADMIN_URL));
        assert!(url.contains("state=abc-123"));
        // redirect_uri 须被编码（http://127.0.0.1:54321/callback → http%3A%2F%2F...）
        assert!(
            url.contains("redirect=http%3A%2F%2F127.0.0.1%3A54321%2Fcallback"),
            "redirect 应被 percent-编码: {url}"
        );
    }

    #[tokio::test]
    async fn save_and_has_account_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = StudioAuthManager::new(tmp.path().to_path_buf());
        assert!(!mgr.has_account("u1").await);
        mgr.save_account("u1", "k1", "tok-secret", Some("张三"))
            .await;
        assert!(mgr.has_account("u1").await);
        assert_eq!(mgr.get_account_name("u1").await.as_deref(), Some("张三"));
        // 重新加载应能读到
        let mgr2 = StudioAuthManager::new(tmp.path().to_path_buf());
        assert!(mgr2.has_account("u1").await);
        assert_eq!(mgr2.get_account_name("u1").await.as_deref(), Some("张三"));
        mgr.remove_account("u1").await;
        assert!(!mgr.has_account("u1").await);
    }

    #[tokio::test]
    async fn refresh_api_key_missing_account_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = StudioAuthManager::new(tmp.path().to_path_buf());
        let err = mgr.refresh_api_key("nobody").await.unwrap_err();
        assert!(matches!(err, StudioAuthError::AccountNotFound(_)));
    }

    #[test]
    fn write_api_key_into_provider_writes_correct_field() {
        let mut provider = Provider {
            id: "p1".into(),
            name: "n".into(),
            settings_config: serde_json::json!({"env": {}}),
            website_url: None,
            category: None,
            created_at: None,
            sort_index: None,
            notes: None,
            meta: Some(ProviderMeta {
                auth_binding: Some(AuthBinding {
                    source: AuthBindingSource::ManagedAccount,
                    auth_provider: Some("studio_account".into()),
                    account_id: Some("u1".into()),
                    needs_relogin: Some(true),
                }),
                ..Default::default()
            }),
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        };

        write_api_key_into_provider(&mut provider, Some("ANTHROPIC_API_KEY"), "new-key-123");
        let env = provider.settings_config["env"].as_object().unwrap();
        assert_eq!(env["ANTHROPIC_API_KEY"].as_str(), Some("new-key-123"));
        assert!(env.get("ANTHROPIC_AUTH_TOKEN").is_none());

        assert!(is_studio_provider(provider.meta.as_ref().unwrap()));
        mark_needs_relogin(&mut provider, false);
        assert_eq!(
            provider.meta.as_ref().unwrap().auth_binding.as_ref().unwrap().needs_relogin,
            Some(false)
        );
    }

    #[test]
    fn write_api_key_into_provider_defaults_to_auth_token() {
        let mut provider = Provider {
            id: "p1".into(),
            name: "n".into(),
            settings_config: serde_json::json!({}),
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
        write_api_key_into_provider(&mut provider, None, "k");
        let env = provider.settings_config["env"].as_object().unwrap();
        assert_eq!(env["ANTHROPIC_AUTH_TOKEN"].as_str(), Some("k"));
        assert!(!is_studio_provider(provider.meta.as_ref().unwrap_or(&ProviderMeta::default())));
    }
}
