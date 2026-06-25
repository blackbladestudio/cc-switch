## Context

CC Switch 将供应商存于本地 SQLite（`~/.cc-switch/`），通过 Tauri 服务层 CRUD 并在切换时写入各 AI 客户端 live 配置。已有能力包括：

- **Per-app Provider**：按应用独立管理
- **Universal Provider**：一份配置同步到 Claude/Codex/Gemini，使用 `merge_json` 深合并保留用户自定义字段
- **WebDAV/S3 整库同步**：全量覆盖式，不适合团队只改供应商而不动设备设置
- **Deeplink**：单次导入，无持续同步

团队场景需要：管理员统一维护 endpoint、模型、协议；成员保留个人 API Key、当前选中供应商、代理开关与配置目录。本设计在 Universal Provider 的「一对多同步 + 深合并」模式上扩展，增加远程 registry 与托管元数据。

## Goals / Non-Goals

**Goals:**

- 定义版本化 Team Provider Registry JSON，支持 HTTP/Git raw URL 拉取（v1）；WebDAV/S3 复用现有 sync 客户端
- 客户端按 provider id 将 registry 条目合并到本地 SQLite，生成/更新 per-app provider
- 团队托管字段覆盖、本地密钥与个人字段保留；冲突可检测并可选择接受团队版或保留本地副本
- 设置页配置 registry URL、手动/自动同步、同步审计；列表展示「团队托管」标识
- 与现有 Universal Provider、WebDAV/S3 整库同步、deeplink 互不破坏

**Non-Goals:**

- 团队 Admin 后台或权限系统（registry 由外部 Git/对象存储维护）
- v1 不支持 `shared_encrypted` 团队共享密钥（仅 `local_required`）
- v1 不覆盖 Claude Desktop、OpenCode、OpenClaw、Hermes（与 Universal Provider 范围一致，后续扩展）
- 集中代理/网关部署（可作为独立 change 后续做）
- OAuth 类供应商（Copilot、Codex OAuth）纳入团队 registry

## Decisions

### 1. 远程配置源：Registry JSON + URL 拉取

**选择**：管理员维护单一 JSON 文件（可放 Git raw、HTTP、WebDAV、S3），客户端拉取并校验 schema version。

**理由**：Git PR 审核、版本历史、CI 校验成本低；比整库同步粒度更细。

**替代方案**：WebDAV 整库同步 — 会覆盖设备设置与冲突难控，拒绝作为团队供应商主路径。

### 2. Provider ID 命名：`team-{app}-{registryId}`

**选择**：registry 中 `id` 为团队级逻辑 ID（如 `team-newapi`），落地到各 app 时使用 `team-claude-{id}`、`team-codex-{id}`、`team-gemini-{id}`，与 Universal 的 `universal-claude-{id}` 对称。

**理由**：避免与成员自建 provider id 冲突；便于按来源批量识别与清理。

### 3. 合并策略：团队字段覆盖 + 密钥保留

**选择**：

| 来源 | 行为 |
|------|------|
| Registry 托管字段（name、baseUrl、models、meta.apiFormat 等） | 覆盖本地同 id provider |
| API Key / auth 相关 env 字段 | 始终保留本地非空值 |
| 成员 notes、sortIndex | 保留本地 |
| `ProviderMeta.teamManaged` | 写入/更新团队元数据 |

合并实现复用 `ProviderService::merge_json()`（Universal sync 已有），并在 merge 前 strip 密钥字段、merge 后 restore。

**替代方案**：全量 replace — 会丢失成员密钥与自定义字段，拒绝。

### 4. 托管元数据扩展 `ProviderMeta`

新增字段（TS + Rust 同步）：

```typescript
teamManaged?: {
  teamId: string;
  registryVersion: number;
  registryUpdatedAt?: string;
  sourceUrl?: string;
  lockedFields?: string[];  // 如 ["settingsConfig.env.ANTHROPIC_BASE_URL", "meta.apiFormat"]
  lastSyncedAt?: number;
  localOverride?: boolean;    // 成员选择保留本地副本时为 true
};
```

UI 根据 `lockedFields` 与 `localOverride` 决定只读与冲突提示。

### 5. Registry 条目 → Per-app Provider 转换

**选择**：新增 `TeamRegistryEntry::to_claude_provider()` 等，逻辑参考 `UniversalProvider::to_claude_provider()`，从 registry 的 `apps`、`baseUrl`、`models`、`meta` 生成 `settings_config`。

**apps 数组**：`["claude","codex","gemini"]` 控制启用哪些子 provider；registry 移除某 app 时删除对应 `team-{app}-{id}`。

### 6. 同步触发与存储

**选择**：

- **配置**：存 `settings.json` → `teamProviderSync: { enabled, sourceUrl, autoSyncIntervalMinutes, lastStatus }`（设备级，不同步到远端）
- **触发**：设置页「立即同步」；可选启动时 + 定时（默认 60 分钟，0 禁用）
- **命令**：`fetch_team_registry`、`apply_team_registry`、`get_team_sync_status`

**理由**：与 WebDAV sync 设置模式一致；registry 内容不进 settings.json。

### 7. 删除与停用策略

**选择**：registry 移除某 provider id 时，本地对应 `team-*` provider 标记 `meta.teamManaged.removed = true`（或 UI 提示「团队已移除」），默认不自动 delete，避免打断当前选中；设置页提供「清理已移除的团队供应商」。

**理由**：静默 delete 可能导致当前 provider 无效；显式清理更安全。

### 8. 冲突处理

**选择**：同步前对比本地 `teamManaged.lastSyncedAt` 与本地托管字段 hash；若成员在同步后修改了 locked 字段且 `localOverride !== true`，写入 `TeamSyncConflict` 列表，UI 提供 per-provider：接受团队 / 保留本地（设 `localOverride=true`）。

### 9. 模块布局

```
src-tauri/src/services/team_provider/
  mod.rs           # fetch, validate, apply
  registry.rs      # schema types
  merge.rs         # field-level merge + secret preserve
  convert.rs       # registry entry -> Provider

src-tauri/src/commands/team_provider.rs

src/lib/api/teamProvider.ts
src/components/settings/TeamProviderSyncPanel.tsx
```

## Risks / Trade-offs

| 风险 | 缓解 |
|------|------|
| Registry URL 被篡改或 MITM | 文档建议 HTTPS + 可选 ETag/sha256 校验；后续可加签名 |
| 团队更新覆盖成员 intentional 本地改动 | 冲突检测 + localOverride + 审计日志 |
| 与 Universal Provider 同 baseUrl 重复 | 文档说明 id 命名空间隔离；UI 可警告重复 endpoint |
| autoSync 频繁拉取失败 | 指数退避；lastStatus 展示错误不弹窗 |
| v1 仅 3 app 范围不足 | proposal 已标注 Non-Goal；schema 预留 apps 扩展 |

## Migration Plan

1. 发版含 team provider 功能，默认关闭（`teamProviderSync.enabled = false`）
2. 团队管理员发布 registry JSON 到 Git/HTTP，文档提供示例与 JSON Schema
3. 成员在设置页填入 URL 并首次同步；填写本地 API Key
4. 无数据库 migration 破坏性变更；新 meta 字段 optional，旧数据兼容
5. 回滚：关闭 team sync + 手动删除 `team-*` provider 或使用「清理已移除」

## Open Questions

- v1 是否支持 registry 内 `commonConfigSnippet` 引用（团队级通用配置片段）— 建议 v2
- 是否在同步成功后自动 `sync_current` 到 live — 建议默认否，仅提示「请切换或同步当前供应商」
- Git 私有仓库鉴权 — v1 仅支持公开 URL 或带 token 的 URL query（文档说明），完整 OAuth 后续
