# Miaoyun Provider

## 基本信息

- 供应商：Miaoyun（秒云）
- 官网：https://www.miaoyun.net.cn
- Base URL：https://maas.miaoyun.net.cn
- API Key：由团队成员在本地填写，不应写入团队 Registry

## 模型协议

| 模型 | 协议 |
| --- | --- |
| `myco-4.8` | `anthropic` |
| `myco-4.6` | `anthropic` |
| `mycs-4.6` | `anthropic` |
| `myog-5.5` | `openai_chat` |
| `deepseek-v4-pro` | `openai_chat` |
| `glm-5.1` | `openai_chat` |

## Team Provider Registry 示例

```json
{
  "id": "miaoyun",
  "name": "Miaoyun",
  "apps": ["claude", "claude-desktop", "codex", "gemini"],
  "baseUrl": "https://maas.miaoyun.net.cn",
  "apiKeyPolicy": "local_required",
  "websiteUrl": "https://www.miaoyun.net.cn",
  "models": {
    "claude": {
      "model": "myco-4.8",
      "haikuModel": "mycs-4.6",
      "sonnetModel": "myco-4.6",
      "opusModel": "myco-4.8"
    },
    "claudeDesktop": {
      "mode": "proxy",
      "modelRoutes": {
        "claude-sonnet-4-6": {
          "model": "myco-4.6",
          "labelOverride": "Miaoyun Sonnet"
        },
        "claude-opus-4-8": {
          "model": "myco-4.8",
          "labelOverride": "Miaoyun Opus"
        },
        "claude-fable-5": {
          "model": "myco-4.8",
          "labelOverride": "Miaoyun Fable"
        },
        "claude-haiku-4-5": {
          "model": "mycs-4.6",
          "labelOverride": "Miaoyun Haiku"
        }
      }
    },
    "codex": {
      "model": "myog-5.5",
      "reasoningEffort": "high"
    },
    "gemini": {
      "model": "myog-5.5"
    }
  },
  "meta": {
    "apiFormat": "openai_chat",
    "modelApiFormats": {
      "myco-4.8": "anthropic",
      "myco-4.6": "anthropic",
      "mycs-4.6": "anthropic",
      "myog-5.5": "openai_chat",
      "deepseek-v4-pro": "openai_chat",
      "glm-5.1": "openai_chat"
    }
  }
}
```

## 使用说明

1. 管理员将上面的 provider 条目加入团队 `team-provider-registry.json` 的 `providers` 数组。
2. 团队成员在 cc-switch 的「团队供应商配置」中填写 Registry URL 并同步。
3. 同步后本地会生成 `team-claude-miaoyun`、`team-claude-desktop-miaoyun`、`team-codex-miaoyun`、`team-gemini-miaoyun`。
4. 成员分别在本地 provider 中填写自己的 Miaoyun API Key。

