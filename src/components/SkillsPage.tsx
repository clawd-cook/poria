import { useEffect } from "react";
import { Puzzle } from "lucide-react";
import { useStore } from "../state/store";
import { listSkills } from "../lib/tauri";
import type { SkillInfo } from "../lib/types";

function SkillCard({ skill }: { skill: SkillInfo }) {
  return (
    <div className="rounded-lg border border-slate-700 bg-slate-800/50 p-4 transition-colors hover:bg-slate-800">
      <div className="mb-2 flex items-start justify-between gap-2">
        <div className="flex items-center gap-2">
          <Puzzle className="h-4 w-4 shrink-0 text-blue-400" />
          <span className="text-sm font-medium text-slate-200">
            {skill.name}
          </span>
        </div>
        <span className="shrink-0 rounded bg-slate-700 px-1.5 py-0.5 text-xs text-slate-400">
          v{skill.version}
        </span>
      </div>
      <p className="mb-3 text-xs leading-relaxed text-slate-400">
        {skill.description || "暂无描述"}
      </p>
      <div className="text-xs text-slate-500">
        ID: {skill.id}
      </div>
    </div>
  );
}

export function SkillsPage() {
  const { state, dispatch } = useStore();

  useEffect(() => {
    listSkills()
      .then((skills) => dispatch({ type: "skillsLoaded", skills }))
      .catch(() => {});
  }, [dispatch]);

  return (
    <div className="h-full overflow-y-auto p-6">
      <div className="mb-6">
        <h2 className="text-xl font-semibold text-slate-100">技能管理</h2>
        <p className="mt-1 text-sm text-slate-400">
          已注册 {state.skills.length} 个技能
        </p>
      </div>

      {state.skills.length === 0 ? (
        <div className="flex h-64 items-center justify-center text-sm text-slate-500">
          暂无已注册的技能
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
          {state.skills.map((skill) => (
            <SkillCard key={skill.id} skill={skill} />
          ))}
        </div>
      )}
    </div>
  );
}
