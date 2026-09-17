import {
  ApiOutlined,
  BranchesOutlined,
  SettingOutlined,
  ToolOutlined,
} from "@ant-design/icons";
import { Layout, Menu } from "antd";

import type { ViewType } from "../lib/types";
import { useStore } from "../state/store";
import { AuthStatus } from "./AuthStatus";
import { ChannelsPage } from "./ChannelsPage";
import { PipelineDetail } from "./PipelineDetail";
import { PipelineSidebar } from "./PipelineSidebar";
import { SettingsPanel } from "./SettingsPanel";
import { SkillsPage } from "./SkillsPage";
import { SubmitBar } from "./SubmitBar";

const { Header, Sider, Content } = Layout;

const NAV_ITEMS: { key: ViewType; label: string; icon: React.ReactNode }[] = [
  { key: "pipeline", label: "Pipeline", icon: <BranchesOutlined /> },
  { key: "skills", label: "技能", icon: <ToolOutlined /> },
  { key: "channels", label: "渠道", icon: <ApiOutlined /> },
];

export function Shell() {
  const { state, dispatch } = useStore();
  const currentView = state.ui.view;

  return (
    <Layout style={{ height: "100vh" }}>
      <Header
        style={{
          display: "flex",
          alignItems: "center",
          padding: 0,
          height: 48,
          lineHeight: "48px",
        }}
      >
        <Menu
          mode="horizontal"
          selectedKeys={[currentView]}
          onClick={({ key }) => dispatch({ type: "viewChanged", view: key as ViewType })}
          items={NAV_ITEMS.map((item) => ({
            key: item.key,
            icon: item.icon,
            label: item.label,
          }))}
          style={{ flex: "none", borderBottom: "none" }}
        />
        {currentView === "pipeline" && (
          <div style={{ flex: 1 }}>
            <SubmitBar />
          </div>
        )}
      </Header>

      <Layout>
        {currentView === "pipeline" && (
          <>
            <Sider width={320} style={{ overflow: "auto" }}>
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  height: "100%",
                }}
              >
                <div style={{ flex: 1, minHeight: 0, overflow: "auto" }}>
                  <PipelineSidebar />
                </div>
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    borderTop: "1px solid rgba(255,255,255,0.08)",
                  }}
                >
                  <AuthStatus />
                  <SettingOutlined
                    onClick={() => dispatch({ type: "settingsToggled", open: true })}
                    style={{ padding: 12, cursor: "pointer", fontSize: 16 }}
                  />
                </div>
              </div>
            </Sider>
            <Content style={{ overflow: "auto" }}>
              <PipelineDetail />
            </Content>
          </>
        )}

        {currentView === "skills" && (
          <Content style={{ overflow: "auto" }}>
            <SkillsPage />
          </Content>
        )}

        {currentView === "channels" && (
          <Content style={{ overflow: "auto" }}>
            <ChannelsPage />
          </Content>
        )}
      </Layout>

      {state.ui.settingsOpen && <SettingsPanel />}
    </Layout>
  );
}
