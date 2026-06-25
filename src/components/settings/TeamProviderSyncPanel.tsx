import { useEffect, useMemo, useState } from "react";
import { Loader2, RefreshCw, Save, Users } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { TeamSyncConflictDialog } from "@/components/settings/TeamSyncConflictDialog";
import {
  useApplyTeamRegistry,
  useCleanupRemovedTeamProviders,
  useSaveTeamProviderSyncSettings,
  useTeamProviderSyncSettings,
  useTeamProviderSyncStatus,
} from "@/hooks/useTeamProviderSync";
import type { TeamProviderSyncSettings } from "@/types";

const DEFAULT_TEAM_REGISTRY_URL =
  "http://inner.blackblade.com/files/configs/team-blackblade-registry.json";

export function TeamProviderSyncPanel() {
  const { t } = useTranslation();
  const { data: savedSettings } = useTeamProviderSyncSettings();
  const { data: syncStatus, refetch: refetchStatus } = useTeamProviderSyncStatus();
  const saveSettings = useSaveTeamProviderSyncSettings();
  const applyRegistry = useApplyTeamRegistry();
  const cleanupRemoved = useCleanupRemovedTeamProviders();

  const [enabled, setEnabled] = useState(false);
  const [sourceUrl, setSourceUrl] = useState("");
  const [autoSyncIntervalMinutes, setAutoSyncIntervalMinutes] = useState(0);
  const [conflictOpen, setConflictOpen] = useState(false);

  useEffect(() => {
    setEnabled(savedSettings?.enabled ?? false);
    setSourceUrl(savedSettings?.sourceUrl || DEFAULT_TEAM_REGISTRY_URL);
    setAutoSyncIntervalMinutes(savedSettings?.autoSyncIntervalMinutes ?? 0);
  }, [savedSettings]);

  useEffect(() => {
    const unlisten = listen("team-provider-synced", () => {
      void refetchStatus();
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [refetchStatus]);

  const pendingConflicts = useMemo(
    () => syncStatus?.pendingConflicts ?? syncStatus?.lastSummary?.conflicts ?? [],
    [syncStatus],
  );

  const buildSettings = (): TeamProviderSyncSettings => ({
    enabled,
    sourceUrl: sourceUrl.trim(),
    autoSyncIntervalMinutes,
    status: savedSettings?.status,
  });

  const handleSave = async () => {
    if (enabled && !sourceUrl.trim()) {
      toast.error(t("teamProvider.missingUrl"));
      return;
    }
    try {
      await saveSettings.mutateAsync(buildSettings());
      toast.success(t("teamProvider.saveSuccess"));
    } catch (error) {
      toast.error(String(error));
    }
  };

  const handleSync = async () => {
    const url = sourceUrl.trim();
    if (!url) {
      toast.error(t("teamProvider.missingUrl"));
      return;
    }
    try {
      if (enabled) {
        await saveSettings.mutateAsync(buildSettings());
      }
      const summary = await applyRegistry.mutateAsync(url);
      const conflicts = summary.conflicts ?? [];
      toast.success(
        t("teamProvider.syncSuccess", {
          created: summary.created ?? 0,
          updated: summary.updated ?? 0,
          conflicts: conflicts.length,
        }),
      );
      if (conflicts.length > 0) {
        setConflictOpen(true);
      }
    } catch (error) {
      toast.error(String(error));
    }
  };

  const handleCleanup = async () => {
    try {
      const count = await cleanupRemoved.mutateAsync();
      toast.success(t("teamProvider.cleanupSuccess", { count }));
    } catch (error) {
      toast.error(String(error));
    }
  };

  const lastSummary = syncStatus?.lastSummary;
  const lastError = syncStatus?.lastError;
  const lastSummaryConflicts = lastSummary?.conflicts ?? [];

  return (
    <div className="space-y-4">
      <Alert>
        <AlertDescription>{t("teamProvider.firstRunHint")}</AlertDescription>
      </Alert>

      <div className="flex items-center justify-between gap-4">
        <div>
          <div className="font-medium">{t("teamProvider.enabled")}</div>
          <div className="text-sm text-muted-foreground">
            {t("teamProvider.enabledHint")}
          </div>
        </div>
        <Switch checked={enabled} onCheckedChange={setEnabled} />
      </div>

      <div className="space-y-2">
        <label className="text-sm font-medium">{t("teamProvider.sourceUrl")}</label>
        <Input
          value={sourceUrl}
          onChange={(event) => setSourceUrl(event.target.value)}
          placeholder={t("teamProvider.sourceUrlPlaceholder", {
            defaultValue: DEFAULT_TEAM_REGISTRY_URL,
          })}
        />
      </div>

      <div className="space-y-2">
        <label className="text-sm font-medium">
          {t("teamProvider.autoSyncInterval")}
        </label>
        <Input
          type="number"
          min={0}
          value={autoSyncIntervalMinutes}
          onChange={(event) =>
            setAutoSyncIntervalMinutes(Number(event.target.value) || 0)
          }
        />
        <p className="text-xs text-muted-foreground">
          {t("teamProvider.autoSyncIntervalHint")}
        </p>
      </div>

      {lastSummary && (
        <div className="rounded-lg border border-border/60 p-3 text-sm">
          <div className="font-medium">{t("teamProvider.lastSyncTitle")}</div>
          <div className="mt-1 text-muted-foreground">
            {t("teamProvider.lastSyncSummary", {
              created: lastSummary.created ?? 0,
              updated: lastSummary.updated ?? 0,
              skipped: lastSummary.skipped ?? 0,
              removed: lastSummary.removed ?? 0,
              conflicts: lastSummaryConflicts.length,
            })}
          </div>
        </div>
      )}

      {lastError && (
        <div className="rounded-lg border border-destructive/40 bg-destructive/5 p-3 text-sm text-destructive">
          {lastError}
        </div>
      )}

      <div className="flex flex-wrap gap-2">
        <Button
          type="button"
          variant="outline"
          onClick={() => void handleSave()}
          disabled={saveSettings.isPending}
        >
          {saveSettings.isPending ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <Save className="mr-2 h-4 w-4" />
          )}
          {t("teamProvider.save")}
        </Button>
        <Button
          type="button"
          onClick={() => void handleSync()}
          disabled={applyRegistry.isPending}
        >
          {applyRegistry.isPending ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <RefreshCw className="mr-2 h-4 w-4" />
          )}
          {t("teamProvider.syncNow")}
        </Button>
        <Button
          type="button"
          variant="secondary"
          onClick={() => void handleCleanup()}
          disabled={cleanupRemoved.isPending}
        >
          <Users className="mr-2 h-4 w-4" />
          {t("teamProvider.cleanupRemoved")}
        </Button>
        {pendingConflicts.length > 0 && (
          <Button type="button" variant="destructive" onClick={() => setConflictOpen(true)}>
            {t("teamProvider.resolveConflicts", { count: pendingConflicts.length })}
          </Button>
        )}
      </div>

      <TeamSyncConflictDialog
        open={conflictOpen}
        onOpenChange={setConflictOpen}
        conflicts={pendingConflicts}
        sourceUrl={sourceUrl.trim()}
      />
    </div>
  );
}
