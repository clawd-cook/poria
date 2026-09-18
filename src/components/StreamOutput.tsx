import { CheckCircleOutlined, FileTextOutlined, ToolOutlined } from "@ant-design/icons";
import { Collapse, Spin, Typography } from "antd";
import { useEffect, useRef } from "react";

import type { StreamChunk } from "../lib/types";
import { useStore } from "../state/store";

const { Text, Paragraph } = Typography;

export function StreamOutput({ pipelineId }: { pipelineId: string }) {
  const { state } = useStore();
  const chunks = state.streamOutput[pipelineId] ?? [];
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [chunks.length]);

  if (chunks.length === 0) {
    return (
      <div style={{ padding: "16px 0", textAlign: "center" }}>
        <Spin size="small" />
        <Text type="secondary" style={{ marginLeft: 8 }}>
          等待 Claude 输出...
        </Text>
      </div>
    );
  }

  return (
    <div style={{ maxHeight: 384, overflowY: "auto", fontFamily: "monospace", fontSize: 13 }}>
      {chunks.map((chunk, i) => (
        <ChunkLine key={i} chunk={chunk} />
      ))}
      <div ref={bottomRef} />
    </div>
  );
}

function ChunkLine({ chunk }: { chunk: StreamChunk }) {
  switch (chunk.type) {
    case "text":
      return (
        <Paragraph style={{ margin: "2px 0", whiteSpace: "pre-wrap" }}>{chunk.content}</Paragraph>
      );
    case "tool_use":
      return (
        <Collapse
          size="small"
          items={[
            {
              key: "1",
              label: (
                <Text type="secondary">
                  <ToolOutlined style={{ marginRight: 4 }} />
                  {chunk.tool_name ?? "tool"}
                </Text>
              ),
              children: (
                <pre style={{ margin: 0, fontSize: 12, overflowX: "auto" }}>{chunk.content}</pre>
              ),
            },
          ]}
          style={{ marginBottom: 4 }}
        />
      );
    case "tool_result":
      return (
        <Collapse
          size="small"
          items={[
            {
              key: "1",
              label: (
                <Text type="success">
                  <FileTextOutlined style={{ marginRight: 4 }} />
                  Result
                </Text>
              ),
              children: (
                <pre style={{ margin: 0, fontSize: 12, overflowX: "auto" }}>{chunk.content}</pre>
              ),
            },
          ]}
          style={{ marginBottom: 4 }}
        />
      );
    case "result":
      return (
        <Text type="success" strong style={{ display: "block", margin: "4px 0" }}>
          <CheckCircleOutlined style={{ marginRight: 4 }} />
          {chunk.content}
        </Text>
      );
    default:
      return (
        <Text type="secondary" style={{ display: "block" }}>
          {chunk.content}
        </Text>
      );
  }
}
