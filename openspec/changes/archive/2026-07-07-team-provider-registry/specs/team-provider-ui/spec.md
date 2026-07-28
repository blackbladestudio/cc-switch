## ADDED Requirements

### Requirement: 团队 Provider 设置面板

设置 UI SHALL 包含「团队供应商配置」区域，用户可：

- 启用或禁用团队 provider 同步
- 配置 registry 的 `sourceUrl`
- 配置自动同步间隔（分钟，0 表示关闭）
- 手动触发「立即同步」
- 查看最近同步状态（时间、成功/失败、摘要计数）

#### Scenario: 配置 registry URL

- **WHEN** 用户输入合法 HTTPS URL 并保存团队同步设置
- **THEN** URL 持久化到本地设置，并用于后续同步操作

#### Scenario: 展示最近同步结果

- **WHEN** 一次同步完成，结果为 2 条更新、1 条冲突
- **THEN** 设置面板显示时间戳、成功状态及摘要「2 条更新，1 条冲突」

### Requirement: 供应商列表上的团队托管标识

当 `meta.teamManaged` 存在且 `removed` 不为 true 时，provider 列表与详情视图 MUST 显示「团队托管」视觉标识。

`localOverride: true` 的团队托管 provider MUST 额外显示「本地覆盖」标识。

#### Scenario: 团队 provider 显示标识

- **WHEN** provider 设置了 `meta.teamManaged.teamId`
- **THEN** provider 卡片显示团队托管标识

#### Scenario: 本地覆盖标识

- **WHEN** provider 的 `meta.teamManaged.localOverride` 为 true
- **THEN** provider 卡片同时显示团队托管与本地覆盖标识

### Requirement: 表单中锁定字段只读

编辑无 `localOverride` 的团队托管 provider 时，与 `meta.teamManaged.lockedFields` 对应的表单字段 MUST 为只读。

对于 `apiKeyPolicy: local_required`，API Key 字段 MUST 始终可编辑。

#### Scenario: Base URL 只读

- **WHEN** 用户打开无本地覆盖的团队托管 Claude provider 编辑表单
- **THEN** base URL 字段禁用，并显示由团队配置管理的提示

#### Scenario: API Key 保持可编辑

- **WHEN** 用户打开任意团队托管 provider 的编辑表单
- **THEN** API Key 输入框仍可编辑

### Requirement: 冲突解决对话框

当同步报告冲突时，UI MUST 弹出对话框，列出冲突 provider，每条提供「接受团队版本」与「保留本地副本」操作；存在多条冲突时 MUST 提供「全部接受」。

#### Scenario: 单条冲突处理

- **WHEN** 同步返回 `team-claude-newapi` 的一条冲突
- **THEN** 用户可打开冲突对话框，对该 provider 选择接受或保留本地

#### Scenario: 全部接受冲突

- **WHEN** 同步返回三条冲突，用户点击「全部接受」
- **THEN** 所有冲突 provider 更新为 registry 值，并清除 localOverride

### Requirement: 国际化

团队 provider 设置、标识、提示与冲突对话框的所有用户可见文案 MUST 添加到全部支持的语言文件：`zh`、`zh-TW`、`en`、`ja`。

#### Scenario: 简体中文文案存在

- **WHEN** 应用语言设为 `zh`
- **THEN** 团队 provider 设置标签与冲突对话框文案以中文显示
