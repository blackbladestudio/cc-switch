import type { ProviderMeta } from "@/types";

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
