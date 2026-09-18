import { Table, Tag, theme } from "antd";
import type { ColumnsType } from "antd/es/table";

import type { StageDetail, StageEnum } from "../lib/types";
import { STAGE_LABELS } from "../lib/types";
import { PageSectionTitle } from "./PageFrame";

interface GateResult {
  gate: string;
  passed: boolean;
  actual: string;
  threshold: string;
}

interface GateRow extends GateResult {
  stage: StageEnum;
  key: string;
}

function parseGateResults(raw: string | null): GateResult[] {
  if (!raw) return [];
  try {
    return JSON.parse(raw) as GateResult[];
  } catch {
    return [];
  }
}

const columns: ColumnsType<GateRow> = [
  {
    title: "阶段",
    dataIndex: "stage",
    key: "stage",
    render: (stage: StageEnum) => STAGE_LABELS[stage],
  },
  {
    title: "门禁",
    dataIndex: "gate",
    key: "gate",
  },
  {
    title: "结果",
    dataIndex: "passed",
    key: "passed",
    render: (passed: boolean) => (
      <Tag color={passed ? "success" : "error"}>{passed ? "通过" : "未通过"}</Tag>
    ),
  },
  {
    title: "实际值",
    dataIndex: "actual",
    key: "actual",
  },
  {
    title: "阈值",
    dataIndex: "threshold",
    key: "threshold",
  },
];

export function GateResults({ stages }: { stages: StageDetail[] }) {
  const { token } = theme.useToken();
  const allGates: GateRow[] = stages.flatMap((s) => {
    const results = parseGateResults(s.gate_results);
    return results.map((g, i) => ({
      ...g,
      stage: s.name,
      key: `${s.name}-${i}`,
    }));
  });

  if (allGates.length === 0) return null;

  return (
    <div style={{ marginTop: token.margin }}>
      <PageSectionTitle>门禁结果</PageSectionTitle>
      <Table<GateRow> columns={columns} dataSource={allGates} pagination={false} size="small" />
    </div>
  );
}
