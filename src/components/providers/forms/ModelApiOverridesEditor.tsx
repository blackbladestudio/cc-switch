import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { ClaudeApiFormat, ModelApiOverride } from "@/types";

const FOLLOW_DEFAULT = "__default__";

interface ModelApiOverridesEditorProps {
  overrides: Record<string, ModelApiOverride>;
  onChange: (next: Record<string, ModelApiOverride>) => void;
  /** 新增规则时填入的默认协议。 */
  defaultApiFormat: ClaudeApiFormat;
  /** Base URL 输入框占位符（通常用供应商当前 Base URL）。 */
  baseUrlPlaceholder?: string;
  disabled?: boolean;
  /** 可选插槽：渲染在标题与规则列表之间（如默认协议选择器）。 */
  children?: ReactNode;
}

/**
 * 按模型名/前缀覆盖协议与 Base URL 的规则编辑器。
 * 在 Claude Code 与 Claude Desktop 两个表单间共享，避免逻辑与 UI 重复。
 */
export function ModelApiOverridesEditor({
  overrides,
  onChange,
  defaultApiFormat,
  baseUrlPlaceholder,
  disabled = false,
  children,
}: ModelApiOverridesEditorProps) {
  const { t } = useTranslation();
  const entries = Object.entries(overrides);

  const updateRule = (
    index: number,
    nextPattern: string,
    nextOverride: ModelApiOverride,
  ) => {
    const next = [...entries];
    next[index] = [nextPattern.trim(), nextOverride];
    onChange(Object.fromEntries(next));
  };

  const addRule = () => {
    if (Object.prototype.hasOwnProperty.call(overrides, "")) return;
    onChange({
      ...overrides,
      "": { apiFormat: defaultApiFormat, baseUrl: "" },
    });
  };

  const removeRule = (index: number) => {
    onChange(Object.fromEntries(entries.filter((_, i) => i !== index)));
  };

  return (
    <div className="rounded-lg border border-primary/25 bg-primary/5 p-4 space-y-3">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div className="space-y-1">
          <Label>
            {t("providerForm.modelApiFormatsLabel", {
              defaultValue: "按模型覆盖协议",
            })}
          </Label>
          <p className="text-xs leading-relaxed text-muted-foreground">
            {t("providerForm.modelApiFormatsHint", {
              defaultValue:
                "支持精确匹配或前缀匹配（如 gpt-*），匹配的模型将使用指定协议而非默认协议",
            })}
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="h-8 gap-1.5 self-start"
          onClick={addRule}
          disabled={disabled}
        >
          <Plus className="h-3.5 w-3.5" />
          {t("providerForm.modelApiFormatsAdd", { defaultValue: "添加规则" })}
        </Button>
      </div>

      {children}

      {entries.length === 0 ? (
        <div className="rounded-md border border-dashed bg-background/60 px-3 py-2 text-xs text-muted-foreground">
          {t("providerForm.modelApiFormatsEmpty", {
            defaultValue:
              "暂无覆盖规则。默认使用默认协议和请求地址；需要混用协议或 Base URL 时点击添加规则。",
          })}
        </div>
      ) : (
        <div className="space-y-2">
          <div className="hidden grid-cols-[minmax(0,1fr)_180px_minmax(0,1fr)_40px] gap-2 px-1 text-xs font-medium text-muted-foreground md:grid">
            <span>
              {t("providerForm.modelApiFormatsPattern", {
                defaultValue: "模型名或前缀（如 gpt-*）",
              })}
            </span>
            <span>
              {t("providerForm.modelApiFormatsProtocol", {
                defaultValue: "协议",
              })}
            </span>
            <span>
              {t("providerForm.modelApiFormatsBaseUrl", {
                defaultValue: "Base URL",
              })}
            </span>
            <span />
          </div>
          {entries.map(([pattern, override], index) => (
            <div
              key={`${pattern}-${index}`}
              className="grid grid-cols-1 gap-2 md:grid-cols-[minmax(0,1fr)_180px_minmax(0,1fr)_40px]"
            >
              <Input
                value={pattern}
                onChange={(event) =>
                  updateRule(index, event.target.value, override)
                }
                placeholder={t("providerForm.modelApiFormatsPattern", {
                  defaultValue: "模型名或前缀（如 gpt-*）",
                })}
                autoComplete="off"
                disabled={disabled}
              />
              <Select
                value={override.apiFormat ?? FOLLOW_DEFAULT}
                onValueChange={(value) => {
                  const next = { ...override };
                  if (value === FOLLOW_DEFAULT) {
                    delete next.apiFormat;
                  } else {
                    next.apiFormat = value as ClaudeApiFormat;
                  }
                  updateRule(index, pattern, next);
                }}
                disabled={disabled}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={FOLLOW_DEFAULT}>
                    {t("providerForm.modelApiFormatsDefault", {
                      defaultValue: "跟随默认",
                    })}
                  </SelectItem>
                  <SelectItem value="anthropic">Anthropic</SelectItem>
                  <SelectItem value="openai_chat">OpenAI Chat</SelectItem>
                  <SelectItem value="openai_responses">
                    OpenAI Responses
                  </SelectItem>
                  <SelectItem value="gemini_native">Gemini</SelectItem>
                </SelectContent>
              </Select>
              <Input
                value={override.baseUrl ?? ""}
                onChange={(event) =>
                  updateRule(index, pattern, {
                    ...override,
                    baseUrl: event.target.value,
                  })
                }
                placeholder={baseUrlPlaceholder || "https://api.example.com"}
                autoComplete="off"
                disabled={disabled}
              />
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="h-9 w-9 text-muted-foreground hover:text-destructive"
                onClick={() => removeRule(index)}
                disabled={disabled}
                aria-label={t("common.delete", { defaultValue: "删除" })}
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
