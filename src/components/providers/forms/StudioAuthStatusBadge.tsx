import { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { ExternalLink, Loader2, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";

export interface StudioAuthStatusBadgeProps {
  /** 已登录的 accountId（null=未登录） */
  accountId: string | null;
  /** 用户显示名（null=未登录或未知，UI 回退到 accountId） */
  accountName: string | null;
  /** 登录凭证是否失效需重新登录 */
  needsRelogin: boolean;
  /** 跳转到认证中心（未登录/失效时提示用户去登录） */
  onGoToAuthCenter?: () => void;
  /** 刷新成功后回写最新 key 给表单 save 用 */
  onRefreshedKey: (key: string) => void;
}

/**
 * 工作室账号状态徽标（provider 表单「自动获取」模式用）。
 *
 * 不含登录按钮——登录只在认证中心。这里只显示登录状态 + 手动刷新：
 * - 绿点：已登录且 key 刷新成功 → 「已登录（{name}）」
 * - 红点：未登录 / 刷新失败 / needsRelogin → 提示去认证中心
 */
export function StudioAuthStatusBadge({
  accountId,
  accountName,
  needsRelogin,
  onGoToAuthCenter,
  onRefreshedKey,
}: StudioAuthStatusBadgeProps) {
  const { t } = useTranslation();
  const [refreshing, setRefreshing] = useState(false);
  // ok=true 表示最近一次刷新成功（绿点）；false 表示未刷新过/失败/未登录/失效（红点）
  const [ok, setOk] = useState<boolean>(!!accountId && !needsRelogin);

  const handleRefresh = useCallback(async () => {
    if (!accountId) {
      // 未登录，引导去认证中心
      onGoToAuthCenter?.();
      return;
    }
    setRefreshing(true);
    try {
      const newKey = await invoke<string>("auth_studio_refresh", {
        accountId,
      });
      setOk(true);
      onRefreshedKey(newKey);
      toast.success(
        t("studioAuth.refreshSuccess", { defaultValue: "已重新获取 apiKey" }),
      );
    } catch (e) {
      setOk(false);
      const msg = String(e);
      if (msg.includes("needs_relogin")) {
        toast.error(
          t("studioAuth.needsRelogin", {
            defaultValue: "登录已失效，请重新登录",
          }),
        );
      } else {
        toast.error(
          t("studioAuth.refreshFailed", {
            defaultValue: "重新获取失败：{{msg}}",
            msg,
          }),
        );
      }
    } finally {
      setRefreshing(false);
    }
  }, [accountId, onGoToAuthCenter, onRefreshedKey, t]);

  const isRed = !accountId || needsRelogin || !ok;
  const dotClass = isRed ? "bg-red-500" : "bg-green-500";

  let statusText: string;
  if (!accountId) {
    statusText = t("studioAuth.statusNotLoggedInPrompt", {
      defaultValue: "未登录，请前往认证中心登录",
    });
  } else if (needsRelogin) {
    statusText = t("studioAuth.statusRelogin", {
      defaultValue: "登录失效，请重新登录",
    });
  } else if (ok) {
    const displayName = accountName || accountId;
    statusText = t("studioAuth.statusLoggedIn", {
      defaultValue: "已登录（{{name}}）",
      name: displayName,
    });
  } else {
    // 有 accountId 且未失效，但还没刷新拿到 key
    const displayName = accountName || accountId;
    statusText = t("studioAuth.statusPendingRefresh", {
      defaultValue: "已登录（{{name}}），点击刷新获取 key",
      name: displayName,
    });
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center gap-2 rounded-md border border-border p-2">
        <span className={`h-2 w-2 shrink-0 rounded-full ${dotClass}`} />
        <span className="text-sm text-muted-foreground">{statusText}</span>
        <div className="ml-auto flex items-center gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={handleRefresh}
            disabled={refreshing}
          >
            {refreshing ? (
              <Loader2 className="mr-1 h-3 w-3 animate-spin" />
            ) : (
              <RefreshCw className="mr-1 h-3 w-3" />
            )}
            {t("studioAuth.refresh", { defaultValue: "刷新" })}
          </Button>
          {isRed ? (
            <Button
              type="button"
              variant="link"
              size="sm"
              className="h-7 px-2 text-xs"
              onClick={() => onGoToAuthCenter?.()}
            >
              {t("studioAuth.goToAuthCenter", {
                defaultValue: "前往认证中心",
              })}
              <ExternalLink className="ml-1 h-3 w-3" />
            </Button>
          ) : null}
        </div>
      </div>
    </div>
  );
}

export default StudioAuthStatusBadge;
