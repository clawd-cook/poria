import { Card, Flex, Statistic, Table, theme } from "antd";
import { useEffect, useState } from "react";

import { getPipelineStats } from "../lib/tauri";
import { STAGE_LABELS, type ObservabilitySummary, type StageEnum } from "../lib/types";
import { PageSectionTitle } from "./PageFrame";

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
  const { token } = theme.useToken();
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
      <Card size="small">
        <PageSectionTitle>本机流水线汇总</PageSectionTitle>
        暂无本地流水线数据。
      </Card>
    );
  }

  const stats = (
    <Flex gap={token.marginLG} wrap="wrap">
      <Statistic title="流水线" value={summary.pipeline_total} />
      <Statistic title="成功率" value={formatPercent(summary.success_rate)} />
      <Statistic title="人工介入" value={formatPercent(summary.hitl_rate)} />
      <Statistic title="费用" value={formatUsd(summary.cost_usd_total)} />
    </Flex>
  );

  if (compact) {
    return (
      <Card size="small" styles={{ body: { padding: token.paddingSM } }}>
        {stats}
      </Card>
    );
  }

  return (
    <Card size="small">
      <PageSectionTitle>本机流水线汇总</PageSectionTitle>
      {stats}
      <Table
        columns={[
          {
            dataIndex: "stage",
            key: "stage",
            render: (stage: string) => STAGE_LABELS[stage as StageEnum] ?? stage,
            title: "阶段",
          },
          { dataIndex: "completed", key: "completed", title: "完成" },
          { dataIndex: "failed", key: "failed", title: "失败" },
          { dataIndex: "blocked", key: "blocked", title: "阻断" },
          {
            dataIndex: "success_rate",
            key: "success_rate",
            render: (value: number | null) => formatPercent(value),
            title: "成功率",
          },
          {
            dataIndex: "duration_p50_ms",
            key: "duration_p50_ms",
            render: (value: number | null) => formatDuration(value),
            title: "耗时 p50",
          },
          {
            dataIndex: "duration_p95_ms",
            key: "duration_p95_ms",
            render: (value: number | null) => formatDuration(value),
            title: "耗时 p95",
          },
          {
            dataIndex: "cost_usd",
            key: "cost_usd",
            render: (value: number) => formatUsd(value),
            title: "费用",
          },
        ]}
        dataSource={summary.stages}
        pagination={false}
        rowKey="stage"
        size="small"
        style={{ marginTop: token.marginMD }}
      />
    </Card>
  );
}

export { formatDuration, formatPercent, formatUsd };
