import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Loader2, LogOut, RefreshCw, Sparkles, Sword } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Label } from "@/components/ui/label";
import { useStudioLogin } from "@/components/providers/forms/hooks/useStudioLogin";
import type { StudioAccountStatus } from "@/types";

/**
 * 工作室账号 OAuth 认证区块（认证中心用，无 props 全局组件）。
 *
 * 行为：
 * - 挂载时拉取已登录账号列表
 * - 未登录：显示「登录」按钮 → useStudioLogin.startLogin（打开后台浏览器）
 * - 已登录：显示账号列表 + 「重新获取」(refresh) + 「登出」(remove)
 * - 登录成功/登出后刷新列表
 */
export function StudioAuthCenterSection() {
  const { t } = useTranslation();
  const [accounts, setAccounts] = useState<StudioAccountStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshingId, setRefreshingId] = useState<string | null>(null);
  const [removingId, setRemovingId] = useState<string | null>(null);

  const refreshList = useCallback(async () => {
    try {
      const list = await invoke<StudioAccountStatus[]>(
        "auth_studio_list_accounts",
      );
      setAccounts(list);
    } catch (e) {
      console.warn("[StudioAuth] 拉取账号列表失败", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refreshList();
  }, [refreshList]);

  const { startLogin, pendingState } = useStudioLogin({
    onSuccess: () => {
      refreshList();
    },
  });

  const handleRefresh = useCallback(
    async (accountId: string) => {
      setRefreshingId(accountId);
      try {
        await invoke<string>("auth_studio_refresh", { accountId });
        toast.success(
          t("studioAuth.refreshSuccess", { defaultValue: "已重新获取 apiKey" }),
        );
      } catch (e) {
        const msg = String(e);
        if (msg.includes("needs_relogin")) {
          toast.error(
            t("studioAuth.needsRelogin", {
              defaultValue: "登录已失效，请重新登录",
            }),
          );
          // 标记该账号 needsRelogin
          setAccounts((prev) =>
            prev.map((a) =>
              a.accountId === accountId ? { ...a, needsRelogin: true } : a,
            ),
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
        setRefreshingId(null);
      }
    },
    [t],
  );

  const handleLogout = useCallback(
    async (accountId: string) => {
      setRemovingId(accountId);
      try {
        await invoke("auth_studio_remove_account", { accountId });
        toast.success(
          t("studioAuth.logoutSuccess", { defaultValue: "已登出" }),
        );
        await refreshList();
      } catch (e) {
        toast.error(
          t("studioAuth.logoutFailed", {
            defaultValue: "登出失败：{{msg}}",
            msg: String(e),
          }),
        );
      } finally {
        setRemovingId(null);
      }
    },
    [refreshList, t],
  );

  const hasAnyAccount = accounts.length > 0;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <Label>
          {t("studioAuth.authStatus", { defaultValue: "认证状态" })}
        </Label>
        <Badge
          variant={hasAnyAccount ? "default" : "secondary"}
          className={hasAnyAccount ? "bg-green-500 hover:bg-green-600" : ""}
        >
          {hasAnyAccount
            ? t("studioAuth.accountCount", {
                count: accounts.length,
                defaultValue: `${accounts.length} 个账号`,
              })
            : t("studioAuth.notAuthenticated", { defaultValue: "未认证" })}
        </Badge>
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-4 text-sm text-muted-foreground">
          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          {t("common.loading", { defaultValue: "加载中…" })}
        </div>
      ) : null}

      {/* 已登录账号列表 */}
      {hasAnyAccount ? (
        <div className="space-y-2">
          <Label className="text-sm text-muted-foreground">
            {t("studioAuth.loggedInAccounts", { defaultValue: "已登录账号" })}
          </Label>
          <div className="space-y-1">
            {accounts.map((account) => {
              const displayName = account.accountName || account.accountId;
              return (
                <div
                  key={account.accountId}
                  className="flex items-center justify-between rounded-md border bg-muted/30 p-2"
                >
                  <div className="flex items-center gap-2">
                    <Sword className="h-5 w-5 text-muted-foreground" />
                    <span className="text-sm font-medium">{displayName}</span>
                    {account.needsRelogin ? (
                      <Badge variant="destructive" className="text-xs">
                        {t("studioAuth.reloginNeeded", {
                          defaultValue: "需重新登录",
                        })}
                      </Badge>
                    ) : null}
                  </div>
                  <div className="flex items-center gap-1">
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="h-7 px-2 text-xs text-muted-foreground"
                      onClick={() => handleRefresh(account.accountId)}
                      disabled={refreshingId === account.accountId}
                    >
                      {refreshingId === account.accountId ? (
                        <Loader2 className="mr-1 h-3 w-3 animate-spin" />
                      ) : (
                        <RefreshCw className="mr-1 h-3 w-3" />
                      )}
                      {t("studioAuth.refresh", { defaultValue: "刷新" })}
                    </Button>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="h-7 px-2 text-xs text-red-500 hover:text-red-600"
                      onClick={() => handleLogout(account.accountId)}
                      disabled={removingId === account.accountId}
                    >
                      {removingId === account.accountId ? (
                        <Loader2 className="h-3 w-3 animate-spin" />
                      ) : (
                        <LogOut className="h-3 w-3" />
                      )}
                    </Button>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      ) : null}

      {/* 登录按钮：仅未登录时显示（单账号，已登录需先登出再登录其他账号） */}
      {!loading && !hasAnyAccount ? (
        <Button
          type="button"
          onClick={startLogin}
          variant="default"
          className="w-full"
          disabled={pendingState !== null}
        >
          {pendingState !== null ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <Sparkles className="mr-2 h-4 w-4" />
          )}
          {pendingState
            ? t("studioAuth.waiting", { defaultValue: "等待登录…" })
            : t("studioAuth.login", { defaultValue: "登录工作室账号" })}
        </Button>
      ) : null}
    </div>
  );
}

export default StudioAuthCenterSection;
