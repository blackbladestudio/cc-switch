import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { teamProviderApi } from "@/lib/api/teamProvider";
import type { TeamProviderSyncSettings } from "@/types";

const TEAM_SYNC_SETTINGS_KEY = ["teamProviderSync", "settings"] as const;
const TEAM_SYNC_STATUS_KEY = ["teamProviderSync", "status"] as const;

export function useTeamProviderSyncSettings() {
  return useQuery({
    queryKey: TEAM_SYNC_SETTINGS_KEY,
    queryFn: () => teamProviderApi.getSyncSettings(),
  });
}

export function useTeamProviderSyncStatus() {
  return useQuery({
    queryKey: TEAM_SYNC_STATUS_KEY,
    queryFn: () => teamProviderApi.getSyncStatus(),
  });
}

export function useSaveTeamProviderSyncSettings() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (settings: TeamProviderSyncSettings | null) =>
      teamProviderApi.saveSyncSettings(settings),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: TEAM_SYNC_SETTINGS_KEY });
      queryClient.invalidateQueries({ queryKey: TEAM_SYNC_STATUS_KEY });
    },
  });
}

export function useApplyTeamRegistry() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (sourceUrl: string) => teamProviderApi.applyRegistry(sourceUrl),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: TEAM_SYNC_STATUS_KEY });
      queryClient.invalidateQueries({ queryKey: ["providers"] });
    },
  });
}

export function useResolveTeamSyncConflict() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: teamProviderApi.resolveConflict,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: TEAM_SYNC_STATUS_KEY });
      queryClient.invalidateQueries({ queryKey: ["providers"] });
    },
  });
}

export function useCleanupRemovedTeamProviders() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => teamProviderApi.cleanupRemoved(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["providers"] });
    },
  });
}
