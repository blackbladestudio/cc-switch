import { Loader2, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  useApplyTeamRegistry,
  useTeamProviderSyncSettings,
} from "@/hooks/useTeamProviderSync";
import { cn } from "@/lib/utils";

const DEFAULT_TEAM_REGISTRY_URL =
  "http://inner.blackblade.com/files/configs/team-blackblade-registry.json";

interface TeamProviderSyncButtonProps {
  className?: string;
}

export function TeamProviderSyncButton({
  className,
}: TeamProviderSyncButtonProps) {
  const { t } = useTranslation();
  const { data: settings } = useTeamProviderSyncSettings();
  const applyRegistry = useApplyTeamRegistry();

  if (!settings?.enabled) {
    return null;
  }

  const sourceUrl = settings.sourceUrl?.trim() || DEFAULT_TEAM_REGISTRY_URL;

  const handleSync = async () => {
    try {
      const summary = await applyRegistry.mutateAsync(sourceUrl);
      const conflicts = summary.conflicts ?? [];
      toast.success(
        t("teamProvider.syncSuccess", {
          created: summary.created ?? 0,
          updated: summary.updated ?? 0,
          conflicts: conflicts.length,
        }),
      );
      if (conflicts.length > 0) {
        toast.warning(
          t("teamProvider.syncConflictToast", {
            count: conflicts.length,
            defaultValue: `团队配置同步完成，但有 ${conflicts.length} 个冲突需要处理`,
          }),
        );
      }
    } catch (error) {
      toast.error(String(error));
    }
  };

  return (
    <Button
      type="button"
      variant="ghost"
      size="sm"
      onClick={() => void handleSync()}
      disabled={applyRegistry.isPending}
      className={cn(
        "h-8 gap-1.5 rounded-lg bg-muted/50 px-2 hover:bg-muted",
        className,
      )}
      title={t("teamProvider.topSyncTooltip", {
        defaultValue: "同步团队供应商配置",
      })}
    >
      {applyRegistry.isPending ? (
        <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
      ) : (
        <RefreshCw className="h-4 w-4 text-muted-foreground" />
      )}
      <span className="hidden text-xs font-medium sm:inline">
        {t("teamProvider.topSync", { defaultValue: "团队同步" })}
      </span>
    </Button>
  );
}
