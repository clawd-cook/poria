import { NodeIndexOutlined } from "@ant-design/icons";
import { Badge, Card, Col, Empty, Row, Tag, Typography, theme } from "antd";
import { useEffect } from "react";

import { listChannels } from "../lib/tauri";
import type { ChannelInfo } from "../lib/types";
import { useStore } from "../state/store";
import { PageFrame } from "./PageFrame";

const { Paragraph, Text } = Typography;

function ChannelCard({ channel }: { channel: ChannelInfo }) {
  const { token } = theme.useToken();
  return (
    <Card hoverable size="small">
      <div
        style={{
          alignItems: "flex-start",
          display: "flex",
          justifyContent: "space-between",
          marginBottom: token.marginSM,
        }}
      >
        <Text strong>
          <NodeIndexOutlined style={{ color: token.colorSuccess, marginRight: token.marginXS }} />
          {channel.name}
        </Text>
        <div style={{ alignItems: "center", display: "flex", gap: token.marginXS }}>
          <Badge status="success" />
          <Tag>v{channel.version}</Tag>
        </div>
      </div>
      <Paragraph type="secondary" style={{ marginBottom: token.marginXXS }}>
        {channel.description || "暂无描述"}
      </Paragraph>
      <Text type="secondary">ID: {channel.id}</Text>
    </Card>
  );
}

export function ChannelsPage() {
  const { dispatch, state } = useStore();

  useEffect(() => {
    listChannels()
      .then((channels) => dispatch({ type: "channelsLoaded", channels }))
      .catch(() => {});
  }, [dispatch]);

  return (
    <PageFrame description={`已注册 ${state.channels.length} 个渠道`} title="渠道">
      {state.channels.length === 0 ? (
        <Empty description="暂无已注册的渠道" />
      ) : (
        <Row gutter={[16, 16]}>
          {state.channels.map((channel) => (
            <Col key={channel.id} md={12} xl={8} xs={24}>
              <ChannelCard channel={channel} />
            </Col>
          ))}
        </Row>
      )}
    </PageFrame>
  );
}
