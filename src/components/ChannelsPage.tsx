import { NodeIndexOutlined } from "@ant-design/icons";
import { Badge, Card, Col, Empty, Row, Tag, Typography } from "antd";
import { useEffect } from "react";

import { listChannels } from "../lib/tauri";
import type { ChannelInfo } from "../lib/types";
import { useStore } from "../state/store";

const { Title, Text, Paragraph } = Typography;

function ChannelCard({ channel }: { channel: ChannelInfo }) {
  return (
    <Card hoverable size="small">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: 8 }}>
        <Text strong>
          <NodeIndexOutlined style={{ marginRight: 6, color: "#52c41a" }} />
          {channel.name}
        </Text>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <Badge status="success" />
          <Tag>v{channel.version}</Tag>
        </div>
      </div>
      <Paragraph type="secondary" style={{ fontSize: 12, marginBottom: 4 }}>
        {channel.description || "暂无描述"}
      </Paragraph>
      <Text type="secondary" style={{ fontSize: 12 }}>
        ID: {channel.id}
      </Text>
    </Card>
  );
}

export function ChannelsPage() {
  const { state, dispatch } = useStore();

  useEffect(() => {
    listChannels()
      .then((channels) => dispatch({ type: "channelsLoaded", channels }))
      .catch(() => {});
  }, [dispatch]);

  return (
    <div style={{ padding: 24, height: "100%", overflow: "auto" }}>
      <Title level={4}>渠道管理</Title>
      <Text type="secondary">已注册 {state.channels.length} 个渠道</Text>

      {state.channels.length === 0 ? (
        <Empty description="暂无已注册的渠道" style={{ marginTop: 64 }} />
      ) : (
        <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
          {state.channels.map((channel) => (
            <Col key={channel.id} xs={24} md={12} xl={8}>
              <ChannelCard channel={channel} />
            </Col>
          ))}
        </Row>
      )}
    </div>
  );
}
