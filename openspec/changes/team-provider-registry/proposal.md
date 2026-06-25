## Why

CC Switch 目前以本地 SQLite 为供应商配置的单点真相，团队若要统一维护 endpoint、模型、协议等供应商参数，只能依赖 WebDAV/S3 整库同步、deeplink 单次导入或手动复制，无法在不覆盖个人 API Key、当前选中供应商和设备级设置的前提下集中更新。随着团队规模扩大，需要一种「管理员维护远程配置、成员客户端受控合并」的机制。

## What Changes

- 新增团队 Provider Registry 远程配置格式（版本化 JSON），管理员可托管 endpoint、模型、协议、图标、说明等团队级字段
- 新增客户端拉取与合并服务：从 HTTP/Git raw/WebDAV/S3 URL 拉取 registry，按 provider id 更新本地 SQLite，保留本地密钥与个人字段
- 扩展 `ProviderMeta`，标记团队托管来源、版本、锁定字段；UI 对锁定字段只读，API Key 默认由成员本地填写
- 新增设置页「团队供应商配置」：配置远程 URL、手动/自动同步、查看同步结果与冲突
- 供应商列表展示「团队托管」标识；团队删除 provider 时提示停用而非静默破坏
- 冲突处理：本地修改了团队托管字段时，提供「接受团队版本 / 保留本地副本」选项
- 不影响现有 WebDAV/S3 整库同步、deeplink 导入、统一供应商（Universal Provider）功能

## Capabilities

### New Capabilities

- `team-provider-registry`: 远程团队 Provider Registry 的 JSON schema、版本校验、以及从 HTTP/WebDAV/S3 拉取配置的能力
- `team-provider-sync`: 客户端将 registry 条目合并到本地 providers 的策略引擎、托管元数据、同步调度、冲突检测与审计记录
- `team-provider-ui`: 设置页团队配置入口、同步状态展示、供应商列表托管标识、冲突解决对话框

### Modified Capabilities

<!-- 无现有 openspec/specs，不涉及已有 capability 的需求变更 -->

## Impact

- **Rust 后端**：新增 `team_provider` 服务模块；扩展 `ProviderMeta`；新增 Tauri commands；复用现有 provider DAO 与 universal sync 逻辑
- **前端**：`src/types.ts` 新类型；设置对话框新区域；供应商列表/表单只读与冲突 UI；i18n 四语言文案
- **数据**：`settings.json` 存团队同步配置（URL、自动同步、最近状态）；SQLite providers 表写入团队托管 provider
- **兼容性**：与 WebDAV/S3 整库同步、设备级 `Settings`、OAuth 类供应商（Copilot 等）互不覆盖；OAuth 供应商默认排除在团队 registry 之外
