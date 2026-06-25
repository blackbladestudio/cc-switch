## ADDED Requirements

### Requirement: Registry 文档 Schema

系统 SHALL 接受 Team Provider Registry JSON 文档，顶层字段包括：`version`（整数，当前为 `1`）、`teamId`（字符串）、`updatedAt`（ISO 8601 字符串）、`providers`（供应商条目数组）。

每条 provider 条目 SHALL 包含：`id`（字符串，registry 内唯一）、`name`（字符串）、`apps`（`"claude" | "claude-desktop" | "codex" | "gemini"` 数组）、`baseUrl`（URL 字符串）、`apiKeyPolicy`（枚举，v1 仅 `local_required`），以及可选的 `models`（各 app 模型配置）、`websiteUrl`、`notes`、`icon`、`iconColor`、`meta`（与 ProviderMeta 兼容且不包含密钥的对象）。

Claude Code（`claude`）与 Claude Desktop（`claude-desktop`）SHALL 作为核心支持目标。若两者配置结构不同，registry SHALL 使用独立 app 段落表达，例如 Claude Code 使用 `models.claude`，Claude Desktop 使用 `models.claudeDesktop` 的 `mode` 与 `modelRoutes`。

系统 MUST 拒绝不支持的 `version` 值或缺少必填字段的 registry 文档，并返回明确的校验错误信息。

#### Scenario: 合法 registry 被接受

- **WHEN** 客户端拉取到 `version: 1`、合法 `teamId` 且至少包含一条合法 provider 条目的 registry JSON
- **THEN** 系统无错误解析该文档，并继续执行 apply 或预览

#### Scenario: 不支持的版本被拒绝

- **WHEN** 客户端拉取到 `version: 99` 的 registry
- **THEN** 系统拒绝该文档，并记录 schema 版本不支持的错误

### Requirement: 远程 Registry 拉取

系统 SHALL 从本地设置（`teamProviderSync.sourceUrl`）中配置的可访问 HTTP 或 HTTPS URL 拉取 registry 文档。

拉取操作 MUST 在 URL 为 `https` 时使用 HTTPS，MUST 在存在时尊重 HTTP 缓存头，MUST 在同步状态中记录拉取时间戳、HTTP 状态码和错误信息。

#### Scenario: HTTP 拉取成功

- **WHEN** 用户触发同步，且 `sourceUrl` 指向可访问的 HTTPS URL 并返回合法 JSON
- **THEN** 系统下载 registry，并在 `lastStatus` 中更新成功时间戳

#### Scenario: URL 不可达

- **WHEN** 用户触发同步，但 URL 不可达或返回非 2xx 状态码
- **THEN** 系统不修改本地 provider，并在同步状态中记录失败原因

### Requirement: Registry 校验规则

系统在 apply 前 MUST 校验每条 provider 条目：

- `id` MUST 匹配模式 `^[a-z0-9][a-z0-9-_]{0,63}$`
- `apps` MUST 非空，且仅包含支持的 app 标识
- `baseUrl` MUST 为合法的 HTTP(S) URL
- `apiKeyPolicy` 在 v1 MUST 为 `local_required`；其他值 MUST 被拒绝
- provider 条目 MUST NOT 在 registry JSON 中包含明文 API Key

#### Scenario: 非法 provider id 被拒绝

- **WHEN** registry 条目的 `id` 包含大写字母或空格
- **THEN** 系统拒绝该条目并报告校验失败，且不 apply 非法条目

#### Scenario: 明文 API Key 被拒绝

- **WHEN** registry 条目在 settings 中包含 `apiKey` 或其他凭证字段
- **THEN** 系统以安全校验错误拒绝整个 registry
