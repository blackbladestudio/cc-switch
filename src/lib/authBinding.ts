import { isMac, isWindows } from "@/lib/platform";
import type { ProviderMeta } from "@/types";

/** 能不能切 agy 的全局登录：和后端 `agy_account::keychain::supported` 一致（macOS 钥匙串、Windows 凭据管理器）。 */
export function canSwitchAgyAccount(): boolean {
  return isMac() || isWindows();
}

export function resolveManagedAccountId(
  meta: ProviderMeta | undefined,
  authProvider: string,
): string | null {
  const binding = meta?.authBinding;

  if (
    binding?.source === "managed_account" &&
    binding.authProvider === authProvider
  ) {
    return binding.accountId ?? null;
  }

  if (authProvider === "github_copilot") {
    return meta?.githubAccountId ?? null;
  }

  return null;
}

/** 该 provider 是否绑定工作室账号自动获取 apiKey */
export function isStudioAccountProvider(
  meta: ProviderMeta | undefined,
): boolean {
  return (
    meta?.authBinding?.source === "managed_account" &&
    meta?.authBinding?.authProvider === "studio_account"
  );
}

/** 工作室账号 accountId（未登录返回 null） */
export function getStudioAccountId(
  meta: ProviderMeta | undefined,
): string | null {
  if (!isStudioAccountProvider(meta)) return null;
  return meta?.authBinding?.accountId ?? null;
}

/** 工作室账号是否需要重新登录（token 失效） */
export function studioNeedsRelogin(meta: ProviderMeta | undefined): boolean {
  return (
    isStudioAccountProvider(meta) && meta?.authBinding?.needsRelogin === true
  );
}
