import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import type { StudioAuthCallbackPayload } from "@/types";

export interface StudioLoginResult {
  apiKey: string;
  accountId: string;
  accountName: string | null;
}

export interface UseStudioLoginOptions {
  /** 登录成功回调（apiKey 已拿到 + 账号已落盘） */
  onSuccess?: (result: StudioLoginResult) => void;
}

/**
 * 工作室账号登录引擎：startLogin(state)、studio-auth-callback 事件监听（按 state 匹配）、
 * 3 分钟超时兜底、auth_studio_save_account 落盘。供认证中心组件和未来其他调用方复用。
 *
 * 注意：登录按钮只在认证中心使用，provider 表单不再触发登录。
 */
export function useStudioLogin(options: UseStudioLoginOptions = {}) {
  const { t } = useTranslation();
  const { onSuccess } = options;
  const [pendingState, setPendingState] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const pendingRef = useRef<string | null>(null);
  const onSuccessRef = useRef(onSuccess);

  useEffect(() => {
    onSuccessRef.current = onSuccess;
  }, [onSuccess]);

  const startLogin = useCallback(async () => {
    setError(null);
    try {
      const state = crypto.randomUUID();
      pendingRef.current = state;
      setPendingState(state);
      const url = await invoke<string>("auth_studio_login_start", { state });
      await invoke("open_external", { url });
    } catch (e) {
      setPendingState(null);
      pendingRef.current = null;
      const msg = String(e);
      setError(msg);
      toast.error(
        t("studioAuth.startFailed", {
          defaultValue: "启动登录失败：{{msg}}",
          msg,
        }),
      );
    }
  }, [t]);

  // 监听 studio-auth-callback 事件，按 state 匹配
  useEffect(() => {
    const unlisten = listen<StudioAuthCallbackPayload>(
      "studio-auth-callback",
      (event) => {
        const payload = event.payload;
        if (payload.state !== pendingRef.current) return;
        pendingRef.current = null;
        setPendingState(null);

        if (payload.error) {
          setError(payload.error);
          toast.error(
            t("studioAuth.loginFailed", {
              defaultValue: "登录失败：{{msg}}",
              msg: payload.error,
            }),
          );
          return;
        }
        if (!payload.apiKey || !payload.accountId) {
          const msg = t("studioAuth.invalidCallback", {
            defaultValue: "回调缺少 apiKey 或 accountId",
          });
          setError(msg);
          toast.error(msg);
          return;
        }
        // 落盘 token + keyId + 显示名（apiKey 直接交给前端，启动时用 token 静默 reveal 刷新）
        if (payload.token && payload.keyId) {
          invoke("auth_studio_save_account", {
            accountId: payload.accountId,
            keyId: payload.keyId,
            token: payload.token,
            accountName: payload.accountName ?? null,
          }).catch((e) => {
            console.warn("[StudioAuth] 保存 token 失败", e);
          });
        }
        setError(null);
        onSuccessRef.current?.({
          apiKey: payload.apiKey,
          accountId: payload.accountId,
          accountName: payload.accountName ?? null,
        });
        toast.success(
          t("studioAuth.loginSuccess", { defaultValue: "登录成功" }),
        );
      },
    );
    return () => {
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, [t]);

  // 登录超时兜底：与后端 server 超时（3 分钟）对齐。
  // 正常情况下后端超时会先 emit error 事件；此计时器仅防事件未达时 UI 卡在「等待登录…」。
  useEffect(() => {
    if (!pendingState) return;
    const timer = setTimeout(
      () => {
        if (pendingRef.current === pendingState) {
          pendingRef.current = null;
          setPendingState(null);
          const msg = t("studioAuth.timeout", {
            defaultValue: "登录超时，请重试",
          });
          setError(msg);
          toast.error(msg);
        }
      },
      3 * 60 * 1000,
    );
    return () => clearTimeout(timer);
  }, [pendingState, t]);

  return { startLogin, pendingState, error };
}
