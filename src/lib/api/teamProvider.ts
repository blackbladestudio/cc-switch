import { invoke } from "@tauri-apps/api/core";
import type {
  TeamProviderRegistry,
  TeamProviderSyncSettings,
  TeamSyncApplySummary,
  TeamSyncStatus,
} from "@/types";

export const teamProviderApi = {
  async getSyncSettings(): Promise<TeamProviderSyncSettings | null> {
    return (await invoke(
      "get_team_sync_settings",
    )) as TeamProviderSyncSettings | null;
  },

  async saveSyncSettings(
    settings: TeamProviderSyncSettings | null,
  ): Promise<void> {
    await invoke("save_team_sync_settings", { settings });
  },

  async getSyncStatus(): Promise<TeamSyncStatus> {
    return (await invoke("get_team_sync_status")) as TeamSyncStatus;
  },

  async fetchRegistry(sourceUrl: string): Promise<TeamProviderRegistry> {
    return (await invoke("fetch_team_registry", {
      sourceUrl,
    })) as TeamProviderRegistry;
  },

  async applyRegistry(sourceUrl: string): Promise<TeamSyncApplySummary> {
    return (await invoke("apply_team_registry", {
      sourceUrl,
    })) as TeamSyncApplySummary;
  },

  async resolveConflict(input: {
    providerId: string;
    app: string;
    acceptTeam: boolean;
    sourceUrl: string;
  }): Promise<void> {
    await invoke("resolve_team_sync_conflict", {
      request: {
        providerId: input.providerId,
        app: input.app,
        acceptTeam: input.acceptTeam,
        sourceUrl: input.sourceUrl,
      },
    });
  },

  async cleanupRemoved(): Promise<number> {
    return (await invoke("cleanup_removed_team_providers")) as number;
  },
};
