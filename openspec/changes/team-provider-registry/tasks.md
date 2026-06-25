## 1. Schema 与类型

- [x] 1.1 在 `src/types.ts` 中新增 `TeamProviderRegistry`、`TeamRegistryEntry`、`TeamProviderSyncSettings`、`TeamSyncStatus`、`TeamManagedMeta`
- [x] 1.2 扩展 `Settings` 接口，增加可选字段 `teamProviderSync`
- [x] 1.3 在 `src-tauri/src/services/team_provider/registry.rs` 中新增与 registry schema 对应的 Rust 结构体
- [x] 1.4 在 `src-tauri/src/provider.rs` 和 `src/types.ts` 的 `ProviderMeta` 中扩展 `teamManaged` 块
- [x] 1.5 在 `docs/examples/team-provider-registry.json` 下添加示例 registry JSON 与 JSON Schema

## 2. 后端 — 拉取与校验

- [x] 2.1 创建 `src-tauri/src/services/team_provider/mod.rs` 模块骨架，并在 `lib.rs` 中注册
- [x] 2.2 在 `fetch.rs` 中实现 HTTP(S) 拉取（含超时、状态码检查、ETag 支持）
- [x] 2.3 实现 registry 校验（version、id 格式、apps、baseUrl、禁止明文密钥）
- [x] 2.4 通过现有 settings 服务持久化同步状态

## 3. 后端 — 转换与合并

- [x] 3.1 在 `convert.rs` 中实现 `TeamRegistryEntry::to_claude_provider()` / `to_claude_desktop_provider()` / `to_codex_provider()` / `to_gemini_provider()`（参考 Universal Provider 转换逻辑）
- [x] 3.2 在 `merge.rs` 中实现字段级合并并保留 API Key（复用 `ProviderService::merge_json` 模式）
- [x] 3.3 实现 apply 逻辑：按 `team-{app}-{registryId}` 创建/更新各 app 的 provider
- [x] 3.4 实现移除检测：registry 中缺失的 id 对应设置 `meta.teamManaged.removed = true`
- [x] 3.5 实现冲突检测：对比锁定字段 hash 与 `lastSyncedAt`
- [x] 3.6 新增 `cleanup_removed_team_providers` 命令（跳过当前选中的 provider）

## 4. 后端 — Tauri 命令

- [x] 4.1 新增 `get_team_sync_settings` / `save_team_sync_settings` 命令
- [x] 4.2 新增 `fetch_team_registry` 命令（仅下载 + 校验）
- [x] 4.3 新增 `apply_team_registry` 命令（拉取 + 合并 + 返回摘要）
- [x] 4.4 新增 `resolve_team_sync_conflict` 命令（按 provider id 接受团队版 / 保留本地）
- [x] 4.5 新增 `get_team_sync_status` 命令
- [x] 4.6 在 `lib.rs` invoke handler 中注册上述命令

## 5. 后端 — 自动同步

- [x] 5.1 当 `autoSyncIntervalMinutes > 0` 时，在应用启动时启动后台定时任务
- [x] 5.2 对连续拉取失败应用指数退避
- [x] 5.3 发送 Tauri 事件 `team-provider-synced`，携带摘要 payload 供 UI 刷新

## 6. 前端 — API 层

- [x] 6.1 创建 `src/lib/api/teamProvider.ts`，封装上述 Tauri 命令
- [x] 6.2 为同步状态与设置添加 React Query hooks（或沿用现有模式）

## 7. 前端 — 设置 UI

- [x] 7.1 创建 `src/components/settings/TeamProviderSyncPanel.tsx`（启用开关、URL 输入、间隔、同步按钮、状态展示）
- [x] 7.2 将面板集成到现有设置对话框（与 WebDAV/S3 同步区域并列）
- [x] 7.3 添加首次使用提示，说明团队托管字段与个人 API Key 的区别

## 8. 前端 — 供应商列表与表单

- [x] 8.1 在供应商列表卡片上添加「团队托管」与「本地覆盖」标识
- [x] 8.2 根据 `meta.teamManaged.lockedFields` 在 `ProviderForm` / 各 app 表单中将锁定字段设为只读
- [x] 8.3 团队托管 provider 的 API Key 字段保持可编辑
- [x] 8.4 创建 `TeamSyncConflictDialog.tsx`，用于同步后冲突处理
- [x] 8.5 将 Claude Desktop 作为核心团队配置目标接入顶部同步与团队托管只读字段

## 9. 国际化

- [x] 9.1 在 `src/i18n/locales/zh.json` 中添加 `teamProvider.*` 键
- [x] 9.2 在 `src/i18n/locales/zh-TW.json` 中添加 `teamProvider.*` 键
- [x] 9.3 在 `src/i18n/locales/en.json` 中添加 `teamProvider.*` 键
- [x] 9.4 在 `src/i18n/locales/ja.json` 中添加 `teamProvider.*` 键

## 10. 测试与文档

- [x] 10.1 Rust 单元测试：registry 校验、id 映射、密钥保留、冲突检测
- [x] 10.2 Rust 单元测试：移除标记与 cleanup 跳过当前 provider 逻辑
- [x] 10.3 手动测试清单：首次同步、重同步保留密钥、冲突接受/覆盖、registry 移除 app
- [x] 10.4 在 `docs/user-manual/zh/` 下添加团队供应商配置说明章节

## 11. 验证

- [x] 11.1 对 team_provider 模块运行 `cargo test`
- [x] 11.2 运行前端 typecheck/lint
- [x] 11.3 确认 WebDAV/S3 整库同步与 Universal Provider 流程不受影响
