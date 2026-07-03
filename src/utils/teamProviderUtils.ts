import type { Provider } from "@/types";

export function isTeamManagedProvider(provider: Provider): boolean {
  const teamManaged = provider.meta?.teamManaged;
  return !!teamManaged?.teamId && !teamManaged.removed;
}

export function hasTeamLocalOverride(provider: Provider): boolean {
  return !!provider.meta?.teamManaged?.localOverride;
}

export function isTeamManagedReadOnly(provider?: Provider | null): boolean {
  // 团队管理的 provider 历史上会全字段置灰，只允许在冲突解决时选"保留本地"
  // 才解锁。但 Claude Desktop 锁了 7 个字段（其他 app 2-3 个），用户连基本
  // 配置都改不了。改为不锁：用户可自由编辑团队下发的字段；后端冲突检测
  // (team_provider::should_report_conflict) 仍会比对 local_fields_hash，
  // 用户改过后下次团队同步会提示冲突（本地 vs 团队二选一），不会静默覆盖。
  void provider;
  return false;
}

export function isTeamLockedField(
  provider: Provider | undefined,
  fieldPath: string,
): boolean {
  if (!isTeamManagedReadOnly(provider)) return false;
  return (
    provider?.meta?.teamManaged?.lockedFields?.includes(fieldPath) ?? false
  );
}
