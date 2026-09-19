import { Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { invokeErrorMessage } from "@/lib/errors";
import { getSkill, listSkills } from "@/lib/tauri";
import type { SkillDetail, SkillInfo } from "@/lib/types";
import { useStore } from "@/state/store";

import { EmptyState } from "./EmptyState";
import { MarkdownView } from "./MarkdownView";
import { PageFrame } from "./PageFrame";
import { Spinner } from "./Spinner";
import { Card, CardContent } from "./ui/card";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "./ui/sheet";

function SkillCard({ onOpen, skill }: { onOpen: (skill: SkillInfo) => void; skill: SkillInfo }) {
  return (
    <Card
      className="hover:border-primary cursor-pointer transition-colors duration-200"
      onClick={() => onOpen(skill)}
    >
      <CardContent className="p-4">
        <p className="mb-2 flex items-start gap-2 font-semibold">
          <Sparkles aria-hidden className="text-primary mt-0.5 size-4" />
          {skill.name}
        </p>
        <p className="text-muted-foreground mb-1 text-sm">{skill.description || "暂无描述"}</p>
        <p className="text-muted-foreground text-xs">ID: {skill.id}</p>
      </CardContent>
    </Card>
  );
}

function SkillDetailDrawer({ onClose, skill }: { onClose: () => void; skill: SkillInfo | null }) {
  const open = skill !== null;
  const [detail, setDetail] = useState<SkillDetail | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!skill) {
      setDetail(null);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setDetail(null);
    void getSkill(skill.id)
      .then((next) => {
        if (!cancelled) {
          setDetail(next);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          toast.error(invokeErrorMessage(error, "无法读取技能详情"));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [skill]);

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
          <SheetTitle>{skill?.name ?? "技能"}</SheetTitle>
        </SheetHeader>
        <div className="min-h-0 flex-1 overflow-auto">
          {loading ? (
            <Spinner />
          ) : detail ? (
            <div className="grid gap-3">
              <p className="text-muted-foreground text-sm">{detail.id}</p>
              {detail.description ? (
                <p className="text-muted-foreground text-sm">{detail.description}</p>
              ) : null}
              {detail.markdown ? (
                <MarkdownView content={detail.markdown} />
              ) : (
                <EmptyState description="该技能没有 SKILL.md" />
              )}
            </div>
          ) : (
            <EmptyState description="无法展示技能详情" />
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}

export function SkillsPage() {
  const { dispatch, state } = useStore();
  const [viewing, setViewing] = useState<SkillInfo | null>(null);

  useEffect(() => {
    let cancelled = false;
    listSkills()
      .then((skills) => {
        if (!cancelled) {
          dispatch({ skills, type: "skillsLoaded" });
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          toast.error(invokeErrorMessage(error, "无法读取随包技能"));
        }
      });
    return () => {
      cancelled = true;
    };
  }, [dispatch]);

  return (
    <PageFrame
      description={`随包 ${state.skills.length} 个 Claude skill，点击卡片查看详情`}
      title="技能"
    >
      {state.skills.length === 0 ? (
        <EmptyState description="暂无随包 Claude skill" />
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
          {state.skills.map((skill) => (
            <SkillCard key={skill.id} onOpen={setViewing} skill={skill} />
          ))}
        </div>
      )}

      <SkillDetailDrawer onClose={() => setViewing(null)} skill={viewing} />
    </PageFrame>
  );
}
