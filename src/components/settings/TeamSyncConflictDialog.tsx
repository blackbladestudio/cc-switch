import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import {
  useApplyTeamRegistry,
  useResolveTeamSyncConflict,
} from "@/hooks/useTeamProviderSync";
import type { TeamSyncConflict } from "@/types";

interface TeamSyncConflictDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  conflicts: TeamSyncConflict[];
  sourceUrl: string;
}

export function TeamSyncConflictDialog({
  open,
  onOpenChange,
  conflicts,
  sourceUrl,
}: TeamSyncConflictDialogProps) {
  const { t } = useTranslation();
  const resolveConflict = useResolveTeamSyncConflict();
  const applyRegistry = useApplyTeamRegistry();
  const [busyId, setBusyId] = useState<string | null>(null);

  const handleResolve = async (
    conflict: TeamSyncConflict,
    acceptTeam: boolean,
  ) => {
    if (!sourceUrl) {
      toast.error(t("teamProvider.missingUrl"));
      return;
    }
    setBusyId(conflict.providerId);
    try {
      await resolveConflict.mutateAsync({
        providerId: conflict.providerId,
        app: conflict.app,
        acceptTeam,
        sourceUrl,
      });
      toast.success(
        acceptTeam
          ? t("teamProvider.acceptTeamSuccess")
          : t("teamProvider.keepLocalSuccess"),
      );
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusyId(null);
    }
  };

  const handleAcceptAll = async () => {
    if (!sourceUrl) {
      toast.error(t("teamProvider.missingUrl"));
      return;
    }
    try {
      for (const conflict of conflicts) {
        await resolveConflict.mutateAsync({
          providerId: conflict.providerId,
          app: conflict.app,
          acceptTeam: true,
          sourceUrl,
        });
      }
      await applyRegistry.mutateAsync(sourceUrl);
      toast.success(t("teamProvider.acceptAllSuccess"));
      onOpenChange(false);
    } catch (error) {
      toast.error(String(error));
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t("teamProvider.conflictTitle")}</DialogTitle>
          <DialogDescription>{t("teamProvider.conflictDescription")}</DialogDescription>
        </DialogHeader>

        <div className="space-y-3 max-h-[50vh] overflow-y-auto">
          {conflicts.map((conflict) => (
            <div
              key={`${conflict.app}:${conflict.providerId}`}
              className="rounded-lg border border-border/60 p-3"
            >
              <div className="font-medium">{conflict.providerId}</div>
              <div className="text-sm text-muted-foreground">
                {conflict.message ?? t("teamProvider.conflictDefaultMessage")}
              </div>
              <div className="mt-3 flex flex-wrap gap-2">
                <Button
                  size="sm"
                  disabled={busyId === conflict.providerId}
                  onClick={() => void handleResolve(conflict, true)}
                >
                  {t("teamProvider.acceptTeam")}
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={busyId === conflict.providerId}
                  onClick={() => void handleResolve(conflict, false)}
                >
                  {t("teamProvider.keepLocal")}
                </Button>
              </div>
            </div>
          ))}
        </div>

        <DialogFooter>
          {conflicts.length > 1 && (
            <Button variant="outline" onClick={() => void handleAcceptAll()}>
              {t("teamProvider.acceptAll")}
            </Button>
          )}
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            {t("common.close", { defaultValue: "关闭" })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
