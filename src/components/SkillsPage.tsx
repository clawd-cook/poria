import { ApiOutlined } from "@ant-design/icons";
import { App, Card, Col, Drawer, Empty, Flex, Row, Spin, Typography, theme } from "antd";
import { useEffect, useState } from "react";

import { invokeErrorMessage } from "../lib/errors";
import { getSkill, listSkills } from "../lib/tauri";
import type { SkillDetail, SkillInfo } from "../lib/types";
import { useStore } from "../state/store";
import { PageFrame } from "./PageFrame";

const { Paragraph, Text } = Typography;

function SkillCard({ onOpen, skill }: { onOpen: (skill: SkillInfo) => void; skill: SkillInfo }) {
  const { token } = theme.useToken();
  return (
    <Card hoverable onClick={() => onOpen(skill)} size="small">
      <div
        style={{
          alignItems: "flex-start",
          display: "flex",
          justifyContent: "space-between",
          marginBottom: token.marginSM,
        }}
      >
        <Text strong>
          <ApiOutlined style={{ color: token.colorPrimary, marginRight: token.marginXS }} />
          {skill.name}
        </Text>
      </div>
      <Paragraph type="secondary" style={{ marginBottom: token.marginXXS }}>
        {skill.description || "暂无描述"}
      </Paragraph>
      <Text type="secondary">ID: {skill.id}</Text>
    </Card>
  );
}

function SkillDetailDrawer({ onClose, skill }: { onClose: () => void; skill: SkillInfo | null }) {
  const { message } = App.useApp();
  const { token } = theme.useToken();
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
        <div style={{ padding: token.paddingXL, textAlign: "center" }}>
          <Spin />
        </div>
      ) : detail ? (
        <Flex gap={token.marginSM} vertical>
          <Text type="secondary">{detail.id}</Text>
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
    <PageFrame
      description={`随包 ${state.skills.length} 个 Claude skill，点击卡片查看详情`}
      title="技能"
    >
      {state.skills.length === 0 ? (
        <Empty description="暂无随包 Claude skill" />
      ) : (
        <Row gutter={[16, 16]}>
          {state.skills.map((skill) => (
            <Col key={skill.id} md={12} xl={8} xs={24}>
              <SkillCard onOpen={setViewing} skill={skill} />
            </Col>
          ))}
        </Row>
      )}

      <SkillDetailDrawer onClose={() => setViewing(null)} skill={viewing} />
    </PageFrame>
  );
}
