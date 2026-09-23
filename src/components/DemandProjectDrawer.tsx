import { useEffect, useState } from "react";

import type { DemandListItem } from "@/lib/types";

import { DemandProjectBrowser } from "./DemandProjectBrowser";
import { Spinner } from "./Spinner";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "./ui/sheet";

export function DemandProjectDrawer({
  demand,
  onClose,
}: {
  demand: DemandListItem | null;
  onClose: () => void;
}) {
  const open = demand !== null;
  const demandCode = demand?.demand_code ?? "";
  const demandId = demand?.id;
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    if (open) {
      setMounted(true);
    }
  }, [open]);

  return (
    <Sheet
      onOpenChange={(next) => {
        if (!next) {
          onClose();
        }
      }}
      open={open}
    >
      <SheetContent className="overflow-hidden">
        <SheetHeader>
          <SheetTitle>{demand ? `${demand.name} 文档` : "项目文档"}</SheetTitle>
        </SheetHeader>
        <div className="min-h-0 flex-1 overflow-auto">
          {!mounted || !demandCode || !open ? (
            <Spinner />
          ) : (
            <DemandProjectBrowser demandCode={demandCode} demandId={demandId} />
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}
