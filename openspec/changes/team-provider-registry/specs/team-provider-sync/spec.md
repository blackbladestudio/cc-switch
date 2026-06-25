## ADDED Requirements

### Requirement: 团队 Provider ID 映射

系统 SHALL 将每条 registry provider 条目映射为各 app 的本地 provider ID，模式为 `team-{app}-{registryId}`，其中 `{app}` 为 `claude`、`codex` 或 `gemini`。

对于条目 `apps` 数组中列出的每个 app，系统 MUST 创建或更新对应的本地 provider。对于未列出的 app，若此前存在对应的 `team-{app}-{registryId}` provider，系统 MUST 删除或标记为已移除。

#### Scenario: 多 app 条目创建三个 provider

- **WHEN** registry 条目 `id: "newapi"` 的 `apps` 为 `["claude","codex","gemini"]`
- **THEN** 系统创建或更新 `team-claude-newapi`、`team-codex-newapi`、`team-gemini-newapi`

#### Scenario: Registry 条目移除某个 app

- **WHEN** 此前已同步的条目从 `apps` 中移除了 `"gemini"`
- **THEN** 系统将 `team-gemini-{id}` 标记为团队已移除，并停止从 registry 更新该 provider

### Requirement: 字段级合并与密钥保留

将 registry 条目 apply 到同 team ID 的已有本地 provider 时，系统 MUST：

- 覆盖团队托管字段：`name`、团队托管部分的 `settingsConfig`（base URL、models）、团队托管 `meta` 字段（`apiFormat`、`modelApiFormats` 等）
- 保留本地非空的 API Key 与 auth 凭证字段
- 保留本地 `notes` 与 `sortIndex`（全新创建时除外）
- 更新 `meta.teamManaged`：`teamId`、`registryVersion`、`registryUpdatedAt`、`sourceUrl`、`lastSyncedAt`、`lockedFields`

系统 MUST NOT 修改设备级设置（`currentProvider*`、`enableLocalProxy`、配置目录覆盖等）。

#### Scenario: 首次同步保留空密钥槽位

- **WHEN** 成员同步新的团队 provider 且尚未填写 API Key
- **THEN** 系统创建带团队托管 endpoint 与 models 的 provider，密钥字段为空供本地填写

#### Scenario: 重同步保留已有 API Key

- **WHEN** 成员重同步，且本地 provider 已有非空 API Key
- **THEN** 团队更新会变更 base URL 与 models，但 API Key 值保持不变

### Requirement: 团队托管元数据

每个团队托管的本地 provider MUST 包含 `meta.teamManaged`，至少含：`teamId`、`registryVersion`、`lastSyncedAt`、`lockedFields`。

当 `meta.teamManaged.localOverride` 为 `true` 时，自动同步 MUST 跳过覆盖该 provider 的团队托管字段，直到用户选择「接受团队版本」。

#### Scenario: 元数据中标记锁定字段

- **WHEN** 团队 provider apply 成功
- **THEN** `meta.teamManaged.lockedFields` 包含 base URL 与 apiFormat 等路径，供 UI 只读展示

#### Scenario: 本地覆盖跳过写入

- **WHEN** 用户在冲突 provider 上将 localOverride 设为 true
- **THEN** 后续自动同步不再覆盖该 provider 的团队托管字段

### Requirement: 同步操作与审计

系统 SHALL 暴露 Tauri 命令：`fetch_team_registry`（仅下载）、`apply_team_registry`（拉取 + 合并）、`get_team_sync_status`。

每次 apply 操作 MUST 记录：来源 URL、registry 的 `updatedAt`、创建/更新/跳过/冲突的 provider 数量，以及错误详情（如有）。

可选自动同步 MUST 通过 `teamProviderSync.autoSyncIntervalMinutes` 配置（0 表示禁用）。

#### Scenario: 从设置页手动同步

- **WHEN** 用户在团队 provider 设置中点击「立即同步」
- **THEN** 系统拉取 registry、执行合并，并返回带计数的摘要

#### Scenario: 按间隔自动同步

- **WHEN** 自动同步已启用且间隔为 60 分钟，应用正在运行
- **THEN** 系统每个间隔最多尝试 apply 一次，并更新最近状态

### Requirement: Registry 中已移除的条目

当某个 provider id 从 registry 中消失时，系统 MUST 将对应本地团队 provider 标记为已移除（`meta.teamManaged.removed: true`），而不是静默删除。

系统 SHALL 提供命令或 UI 操作，用于清理已标记为团队移除且非当前选中 provider 的条目。

#### Scenario: Provider 从 registry 中移除

- **WHEN** 同步执行时，某个此前已知的 registry id 不再存在
- **THEN** 本地该 id 对应的 `team-*` provider 被标记为 removed，并在同步摘要中通知用户

### Requirement: 冲突检测

当团队托管 provider 在 `lastSyncedAt` 之后被本地修改，且修改涉及 `lockedFields` 中的字段时，系统 MUST 检测到冲突。

冲突 MUST 在同步状态中展示，并可通过「接受团队版本」（清除 `localOverride`）或「保留本地副本」（设置 `localOverride: true`）解决。

#### Scenario: 修改 base URL 产生冲突

- **WHEN** 用户在上次同步后本地修改了团队锁定的 base URL，且新 registry 版本也变更了同一字段
- **THEN** 同步报告该 provider 冲突，而不是静默覆盖

#### Scenario: 接受团队版本解决冲突

- **WHEN** 用户对冲突 provider 选择「接受团队版本」
- **THEN** 本地 provider 被 registry 值覆盖，`localOverride` 被清除，`lastSyncedAt` 被更新
