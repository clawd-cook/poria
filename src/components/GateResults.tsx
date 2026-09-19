import type { StageDetail, StageEnum } from "@/lib/types";
import { STAGE_LABELS } from "@/lib/types";

import { PageSectionTitle } from "./PageFrame";
import { Badge } from "./ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "./ui/table";

interface GateResult {
  actual: string;
  gate: string;
  passed: boolean;
  threshold: string;
}

interface GateRow extends GateResult {
  key: string;
  stage: StageEnum;
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
  const allGates: GateRow[] = stages.flatMap((s) => {
    const results = parseGateResults(s.gate_results);
    return results.map((g, i) => ({
      ...g,
      key: `${s.name}-${i}`,
      stage: s.name,
    }));
  });

  if (allGates.length === 0) return null;

  return (
    <div className="mt-4">
      <PageSectionTitle>门禁结果</PageSectionTitle>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>阶段</TableHead>
            <TableHead>门禁</TableHead>
            <TableHead>结果</TableHead>
            <TableHead>实际值</TableHead>
            <TableHead>阈值</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {allGates.map((row) => (
            <TableRow key={row.key}>
              <TableCell>{STAGE_LABELS[row.stage] ?? row.stage}</TableCell>
              <TableCell>{row.gate}</TableCell>
              <TableCell>
                <Badge variant={row.passed ? "success" : "destructive"}>
                  {row.passed ? "通过" : "未通过"}
                </Badge>
              </TableCell>
              <TableCell>{row.actual}</TableCell>
              <TableCell>{row.threshold}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
