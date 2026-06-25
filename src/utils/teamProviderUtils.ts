import type { Provider } from "@/types";

export function isTeamManagedProvider(provider: Provider): boolean {
  const teamManaged = provider.meta?.teamManaged;
  return !!teamManaged?.teamId && !teamManaged.removed;
}

export function hasTeamLocalOverride(provider: Provider): boolean {
  return !!provider.meta?.teamManaged?.localOverride;
}

export function isTeamManagedReadOnly(provider?: Provider | null): boolean {
  if (!provider) return false;
  return isTeamManagedProvider(provider) && !hasTeamLocalOverride(provider);
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
