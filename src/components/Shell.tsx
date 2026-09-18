import {
  ApiOutlined,
  DesktopOutlined,
  FolderOutlined,
  HomeOutlined,
  NodeIndexOutlined,
  SettingOutlined,
  UnorderedListOutlined,
} from "@ant-design/icons";
import { Layout, Menu, Typography, theme } from "antd";
import type { MenuProps } from "antd";
import type { ReactNode } from "react";

import type { ViewType } from "../lib/types";
import { useStore } from "../state/store";
import { AuthStatus } from "./AuthStatus";
import { ChannelsPage } from "./ChannelsPage";
import { DemandListPage } from "./DemandListPage";
import { HomeBoard } from "./HomeBoard";
import { RepoListPage } from "./RepoListPage";
import { SettingsPage } from "./SettingsPage";
import { SkillsPage } from "./SkillsPage";
import { WorkspacePage } from "./WorkspacePage";

const { Content, Sider } = Layout;
const { Text } = Typography;

const VIEW_KEYS: ViewType[] = [
  "channels",
  "demands",
  "home",
  "repos",
  "settings",
  "skills",
  "workspace",
];

const NAV_ITEMS: MenuProps["items"] = [
  { key: "home", icon: <HomeOutlined />, label: "看板" },
  { key: "demands", icon: <UnorderedListOutlined />, label: "需求" },
  {
    children: [
      { key: "channels", icon: <NodeIndexOutlined />, label: "渠道" },
      { key: "skills", icon: <ApiOutlined />, label: "技能" },
      { key: "repos", icon: <FolderOutlined />, label: "仓库" },
      { key: "workspace", icon: <DesktopOutlined />, label: "工作区" },
    ],
    key: "resources",
    label: "资源",
  },
  { key: "settings", icon: <SettingOutlined />, label: "设置" },
];

function isViewType(key: string): key is ViewType {
  return VIEW_KEYS.includes(key as ViewType);
}

function PersistentTab({ active, children }: { active: boolean; children: ReactNode }) {
  return (
    <div
      aria-hidden={!active}
      style={{
        display: active ? "block" : "none",
        height: "100%",
        inset: 0,
        overflow: "auto",
        position: "absolute",
      }}
    >
      {children}
    </div>
  );
}

export function Shell() {
  const { state, dispatch } = useStore();
  const { token } = theme.useToken();
  const currentView = state.ui.view;

  return (
    <Layout
      hasSider
      style={{ background: token.colorBgLayout, height: "100vh", overflow: "hidden" }}
    >
      <Sider
        theme="light"
        width={220}
        style={{
          borderRight: `1px solid ${token.colorBorderSecondary}`,
          height: "100%",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            height: "100%",
          }}
        >
          <div
            style={{
              borderBottom: `1px solid ${token.colorBorderSecondary}`,
              flex: "0 0 auto",
              padding: `${token.paddingMD}px ${token.paddingLG}px`,
            }}
          >
            <Text
              style={{
                color: token.colorText,
                display: "block",
                fontSize: token.fontSizeSM,
                fontWeight: token.fontWeightStrong,
                letterSpacing: "0.22em",
                lineHeight: token.lineHeight,
                textTransform: "uppercase",
              }}
            >
              Poria
            </Text>
          </div>
          <Menu
            defaultOpenKeys={["resources"]}
            inlineIndent={token.padding}
            items={NAV_ITEMS}
            mode="inline"
            onClick={({ key }) => {
              if (isViewType(key)) {
                dispatch({ type: "viewChanged", view: key });
              }
            }}
            selectedKeys={[currentView]}
            style={{ borderInlineEnd: "none", flex: 1, minHeight: 0, overflow: "auto" }}
          />
          <div
            style={{
              borderTop: `1px solid ${token.colorBorderSecondary}`,
              flex: "0 0 auto",
            }}
          >
            <AuthStatus />
          </div>
        </div>
      </Sider>

      <Content
        style={{
          background: token.colorBgLayout,
          flex: 1,
          minHeight: 0,
          overflow: "hidden",
          position: "relative",
        }}
      >
        <PersistentTab active={currentView === "home"}>
          <HomeBoard />
        </PersistentTab>
        <PersistentTab active={currentView === "demands"}>
          <DemandListPage />
        </PersistentTab>
        <PersistentTab active={currentView === "channels"}>
          <ChannelsPage />
        </PersistentTab>
        <PersistentTab active={currentView === "skills"}>
          <SkillsPage />
        </PersistentTab>
        <PersistentTab active={currentView === "repos"}>
          <RepoListPage />
        </PersistentTab>
        <PersistentTab active={currentView === "workspace"}>
          <WorkspacePage />
        </PersistentTab>
        <PersistentTab active={currentView === "settings"}>
          <SettingsPage />
        </PersistentTab>
      </Content>
    </Layout>
  );
}
