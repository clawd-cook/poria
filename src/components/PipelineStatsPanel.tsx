import { useEffect, useState } from "react";

import { getPipelineStats } from "@/lib/tauri";
import { STAGE_LABELS, type ObservabilitySummary, type StageEnum } from "@/lib/types";

import { PageSectionTitle } from "./PageFrame";
import { Card, CardContent } from "./ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "./ui/table";

function formatPercent(value: number | null | undefined): string {
  if (value == null) {
    return "—";
  }
  return `${Math.round(value * 100)}%`;
}

function formatDuration(ms: number | null | undefined): string {
  if (ms == null) {
    return "—";
  }
  if (ms < 1000) {
    return `${ms} ms`;
  }
  const seconds = Math.round(ms / 1000);
  if (seconds < 60) {
    return `${seconds} s`;
  }
  const minutes = Math.floor(seconds / 60);
  const rest = seconds % 60;
  return rest === 0 ? `${minutes} min` : `${minutes} min ${rest} s`;
}

function formatUsd(value: number): string {
  return `$${value.toFixed(2)}`;
}

export function PipelineStatsPanel({ compact = false }: { compact?: boolean }) {
  const [summary, setSummary] = useState<ObservabilitySummary | null>(null);

  useEffect(() => {
    let cancelled = false;
    getPipelineStats()
      .then((data) => {
        if (!cancelled) {
          setSummary(data);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setSummary(null);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!summary || summary.pipeline_total === 0) {
    return compact ? null : (
      <Card>
        <CardContent className="p-4">
          <PageSectionTitle>本机流水线汇总</PageSectionTitle>
          暂无本地流水线数据。
        </CardContent>
      </Card>
    );
  }

  const stats = (
    <div className="flex flex-wrap gap-6">
      <Stat label="流水线" value={String(summary.pipeline_total)} />
      <Stat label="成功率" value={formatPercent(summary.success_rate)} />
      <Stat label="人工介入" value={formatPercent(summary.hitl_rate)} />
      <Stat label="费用" value={formatUsd(summary.cost_usd_total)} />
    </div>
  );

  if (compact) {
    return (
      <Card>
        <CardContent className="p-3">{stats}</CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardContent className="p-4">
        <PageSectionTitle>本机流水线汇总</PageSectionTitle>
        {stats}
        <div className="mt-4">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>阶段</TableHead>
                <TableHead>完成</TableHead>
                <TableHead>失败</TableHead>
                <TableHead>阻断</TableHead>
                <TableHead>成功率</TableHead>
                <TableHead>耗时 p50</TableHead>
                <TableHead>耗时 p95</TableHead>
                <TableHead>费用</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {summary.stages.map((row) => (
                <TableRow key={row.stage}>
                  <TableCell>{STAGE_LABELS[row.stage as StageEnum] ?? row.stage}</TableCell>
                  <TableCell>{row.completed}</TableCell>
                  <TableCell>{row.failed}</TableCell>
                  <TableCell>{row.blocked}</TableCell>
                  <TableCell>{formatPercent(row.success_rate)}</TableCell>
                  <TableCell>{formatDuration(row.duration_p50_ms)}</TableCell>
                  <TableCell>{formatDuration(row.duration_p95_ms)}</TableCell>
                  <TableCell>{formatUsd(row.cost_usd)}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      </CardContent>
    </Card>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-muted-foreground text-xs">{label}</p>
      <p className="font-serif text-xl font-bold">{value}</p>
    </div>
  );
}

export { formatDuration, formatPercent, formatUsd };
