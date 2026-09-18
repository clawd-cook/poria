import { Drawer, Empty, Flex, Menu, Spin, Typography } from "antd";
import { useEffect, useState } from "react";

import { invokeErrorMessage } from "../lib/errors";
import { listDemandProject, readDemandProjectFile } from "../lib/tauri";
import type { DemandListItem, DemandProjectFile } from "../lib/types";
import { PrdReviewEditor } from "./PrdReviewEditor";

const { Text, Title } = Typography;

function fileLabel(name: string): string {
  switch (name) {
    case "PRD.md":
      return "PRD";
    case "BACKEND_TRD.md":
      return "后端 TRD";
    case "PRD_REVIEW.md":
      return "需求评审";
    case "TRD.md":
      return "前端 TRD";
    default:
      return name.replace(/\.md$/i, "");
  }
}

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

  const [files, setFiles] = useState<DemandProjectFile[]>([]);
  const [exists, setExists] = useState(false);
  const [projectDir, setProjectDir] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeFile, setActiveFile] = useState<string | null>(null);
  const [content, setContent] = useState("");
  const [contentLoading, setContentLoading] = useState(false);
  const [Markdown, setMarkdown] = useState<
    typeof import("@ant-design/x-markdown").XMarkdown | null
  >(null);

  useEffect(() => {
    if (!open) {
      return;
    }
    let cancelled = false;
    void import("@ant-design/x-markdown").then((mod) => {
      if (!cancelled) {
        setMarkdown(() => mod.XMarkdown);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [open]);

  useEffect(() => {
    if (!open || !demandCode) {
      return;
    }

    let cancelled = false;
    setLoading(true);
    setError(null);
    setFiles([]);
    setActiveFile(null);
    setContent("");
    setExists(false);

    void (async () => {
      try {
        const project = await listDemandProject({ demandCode, demandId });
        if (cancelled) {
          return;
        }
        setExists(project.exists);
        setFiles(project.files);
        setProjectDir(project.project_dir);
        const review = project.files.find((file) => file.name === "PRD_REVIEW.md");
        setActiveFile(review?.name ?? project.files[0]?.name ?? null);
      } catch (err) {
        if (!cancelled) {
          setError(invokeErrorMessage(err, "项目文档加载失败"));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [demandCode, demandId, open]);

  useEffect(() => {
    if (!open || !demandCode || !activeFile || activeFile === "PRD_REVIEW.md") {
      return;
    }

    let cancelled = false;
    setContentLoading(true);
    void (async () => {
      try {
        const file = await readDemandProjectFile({
          demandCode,
          demandId,
          fileName: activeFile,
        });
        if (!cancelled) {
          setContent(file.content);
        }
      } catch (err) {
        if (!cancelled) {
          setContent("");
          setError(invokeErrorMessage(err, "文档读取失败"));
        }
      } finally {
        if (!cancelled) {
          setContentLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [activeFile, demandCode, demandId, open]);

  return (
    <Drawer
      destroyOnClose
      onClose={onClose}
      open={open}
      size="large"
      title={demand ? `${demand.name} 文档` : "项目文档"}
    >
      {loading ? (
        <div style={{ padding: 48, textAlign: "center" }}>
          <Spin />
        </div>
      ) : error && files.length === 0 ? (
        <Empty description={error} />
      ) : !exists ? (
        <Empty description="尚未导出文档，请先在流水线执行初始化" />
      ) : files.length === 0 ? (
        <Empty description="项目目录为空" />
      ) : (
        <Flex gap={16} style={{ minHeight: 360 }}>
          <Menu
            items={files.map((file) => ({
              key: file.name,
              label: fileLabel(file.name),
            }))}
            onClick={({ key }) => {
              setError(null);
              setActiveFile(key);
            }}
            selectedKeys={activeFile ? [activeFile] : []}
            style={{ flex: "0 0 160px" }}
          />
          <div style={{ flex: 1, minWidth: 0, overflow: "auto" }}>
            {projectDir ? (
              <Text type="secondary" style={{ display: "block", fontSize: 12, marginBottom: 8 }}>
                {projectDir}
              </Text>
            ) : null}
            {activeFile === "PRD_REVIEW.md" ? (
              <PrdReviewEditor demandCode={demandCode} demandId={demandId} />
            ) : error ? (
              <Text type="danger">{error}</Text>
            ) : contentLoading ? (
              <Spin />
            ) : Markdown ? (
              <Markdown content={content} openLinksInNewTab />
            ) : (
              <Title level={5}>{activeFile}</Title>
            )}
          </div>
        </Flex>
      )}
    </Drawer>
  );
}
