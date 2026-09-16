import type { StageDetail } from "../lib/types";
import { STAGE_LABELS } from "../lib/types";

interface GateResult {
  gate: string;
  passed: boolean;
  actual: string;
  threshold: string;
}

function parseGateResults(raw: string | null): GateResult[] {
  if (!raw) return [];
  try {
    return JSON.parse(raw) as GateResult[];
  } catch {
    return [];
  }
}

export function GateResults({ stages }: { stages: StageDetail[] }) {
  const allGates = stages.flatMap((s) => {
    const results = parseGateResults(s.gate_results);
    return results.map((g) => ({ ...g, stage: s.name }));
  });

  if (allGates.length === 0) return null;

  return (
    <div className="rounded-lg bg-slate-800/50 p-3">
      <h4 className="mb-2 text-sm font-medium text-slate-300">门禁结果</h4>
      <table className="w-full text-left text-xs">
        <thead>
          <tr className="border-b border-slate-700 text-slate-500">
            <th className="pr-4 pb-1">阶段</th>
            <th className="pr-4 pb-1">门禁</th>
            <th className="pr-4 pb-1">结果</th>
            <th className="pr-4 pb-1">实际值</th>
            <th className="pb-1">阈值</th>
          </tr>
        </thead>
        <tbody>
          {allGates.map((g, i) => (
            <tr key={i} className="border-b border-slate-700/50">
              <td className="py-1 pr-4 text-slate-400">{STAGE_LABELS[g.stage]}</td>
              <td className="py-1 pr-4 text-slate-300">{g.gate}</td>
              <td className="py-1 pr-4">
                <span
                  className={`rounded px-1.5 py-0.5 text-xs font-medium ${
                    g.passed ? "bg-emerald-600/30 text-emerald-300" : "bg-red-600/30 text-red-300"
                  }`}
                >
                  {g.passed ? "通过" : "未通过"}
                </span>
              </td>
              <td className="py-1 pr-4 text-slate-400">{g.actual}</td>
              <td className="py-1 text-slate-500">{g.threshold}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
