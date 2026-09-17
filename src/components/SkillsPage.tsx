import { ApiOutlined } from "@ant-design/icons";
import { Card, Col, Empty, Row, Tag, Typography } from "antd";
import { useEffect } from "react";

import { listSkills } from "../lib/tauri";
import type { SkillInfo } from "../lib/types";
import { useStore } from "../state/store";

const { Title, Text, Paragraph } = Typography;

function SkillCard({ skill }: { skill: SkillInfo }) {
  return (
    <Card hoverable size="small">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: 8 }}>
        <Text strong>
          <ApiOutlined style={{ marginRight: 6, color: "#1677ff" }} />
          {skill.name}
        </Text>
        <Tag>v{skill.version}</Tag>
      </div>
      <Paragraph type="secondary" style={{ fontSize: 12, marginBottom: 4 }}>
        {skill.description || "暂无描述"}
      </Paragraph>
      <Text type="secondary" style={{ fontSize: 12 }}>
        ID: {skill.id}
      </Text>
    </Card>
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
    <div style={{ padding: 24, height: "100%", overflow: "auto" }}>
      <Title level={4}>技能管理</Title>
      <Text type="secondary">已注册 {state.skills.length} 个技能</Text>

      {state.skills.length === 0 ? (
        <Empty description="暂无已注册的技能" style={{ marginTop: 64 }} />
      ) : (
        <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
          {state.skills.map((skill) => (
            <Col key={skill.id} xs={24} md={12} xl={8}>
              <SkillCard skill={skill} />
            </Col>
          ))}
        </Row>
      )}
    </div>
  );
}
