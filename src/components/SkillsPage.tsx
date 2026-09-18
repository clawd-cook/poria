import { ApiOutlined } from "@ant-design/icons";
import { App, Card, Col, Drawer, Empty, Flex, Row, Spin, Typography } from "antd";
import { useEffect, useState } from "react";

import { invokeErrorMessage } from "../lib/errors";
import { getSkill, listSkills } from "../lib/tauri";
import type { SkillDetail, SkillInfo } from "../lib/types";
import { useStore } from "../state/store";

const { Paragraph, Text, Title } = Typography;

function SkillCard({ onOpen, skill }: { onOpen: (skill: SkillInfo) => void; skill: SkillInfo }) {
  return (
    <Card hoverable onClick={() => onOpen(skill)} size="small">
      <div
        style={{
          alignItems: "flex-start",
          display: "flex",
          justifyContent: "space-between",
          marginBottom: 8,
        }}
      >
        <Text strong>
          <ApiOutlined style={{ color: "#1677ff", marginRight: 6 }} />
          {skill.name}
        </Text>
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

function SkillDetailDrawer({ onClose, skill }: { onClose: () => void; skill: SkillInfo | null }) {
  const { message } = App.useApp();
  const open = skill !== null;
  const [detail, setDetail] = useState<SkillDetail | null>(null);
  const [loading, setLoading] = useState(false);
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
    if (!skill) {
      setDetail(null);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setDetail(null);
    void getSkill(skill.id)
      .then((next) => {
        if (!cancelled) {
          setDetail(next);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          message.error(invokeErrorMessage(error, "无法读取技能详情"));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [message, skill]);

  return (
    <Drawer
      destroyOnClose
      getContainer={() => document.body}
      onClose={onClose}
      open={open}
      size="large"
      styles={{
        body: { overflow: "auto" },
        content: { overflow: "hidden" },
        wrapper: { overflow: "hidden" },
      }}
      title={skill?.name ?? "技能"}
    >
      {loading ? (
        <div style={{ padding: 48, textAlign: "center" }}>
          <Spin />
        </div>
      ) : detail ? (
        <Flex vertical gap={12}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {detail.id}
          </Text>
          {detail.description ? (
            <Paragraph type="secondary" style={{ marginBottom: 0 }}>
              {detail.description}
            </Paragraph>
          ) : null}
          {detail.markdown ? (
            Markdown ? (
              <div style={{ minWidth: 0 }}>
                <Markdown content={detail.markdown} openLinksInNewTab />
              </div>
            ) : (
              <Spin />
            )
          ) : (
            <Empty description="该技能没有 SKILL.md" />
          )}
        </Flex>
      ) : (
        <Empty description="无法展示技能详情" />
      )}
    </Drawer>
  );
}

export function SkillsPage() {
  const { message } = App.useApp();
  const { dispatch, state } = useStore();
  const [viewing, setViewing] = useState<SkillInfo | null>(null);

  useEffect(() => {
    let cancelled = false;
    listSkills()
      .then((skills) => {
        if (!cancelled) {
          dispatch({ type: "skillsLoaded", skills });
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          message.error(invokeErrorMessage(error, "无法读取随包技能"));
        }
      });
    return () => {
      cancelled = true;
    };
  }, [dispatch, message]);

  return (
    <div style={{ height: "100%", padding: 24 }}>
      <Title level={4}>技能管理</Title>
      <Text type="secondary">随包 {state.skills.length} 个 Claude skill，点击卡片查看详情</Text>

      {state.skills.length === 0 ? (
        <Empty description="暂无随包 Claude skill" style={{ marginTop: 64 }} />
      ) : (
        <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
          {state.skills.map((skill) => (
            <Col key={skill.id} md={12} xl={8} xs={24}>
              <SkillCard onOpen={setViewing} skill={skill} />
            </Col>
          ))}
        </Row>
      )}

      <SkillDetailDrawer onClose={() => setViewing(null)} skill={viewing} />
    </div>
  );
}
