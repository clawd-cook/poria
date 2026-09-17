import {
  FolderOutlined,
  HomeOutlined,
  SettingOutlined,
  UnorderedListOutlined,
} from "@ant-design/icons";
import { Layout, Menu } from "antd";
import type { ReactNode } from "react";

import type { ViewType } from "../lib/types";
import { useStore } from "../state/store";
import { AuthStatus } from "./AuthStatus";
import { DemandListPage } from "./DemandListPage";
import { HomeBoard } from "./HomeBoard";
import { RepoListPage } from "./RepoListPage";
import { SettingsPage } from "./SettingsPage";

const { Header, Content } = Layout;

const NAV_ITEMS: { icon: ReactNode; key: ViewType; label: string }[] = [
  { key: "home", icon: <HomeOutlined />, label: "首页" },
  { key: "demands", icon: <UnorderedListOutlined />, label: "需求列表" },
  { key: "repos", icon: <FolderOutlined />, label: "仓库列表" },
  { key: "settings", icon: <SettingOutlined />, label: "设置" },
];

function isViewType(key: string): key is ViewType {
  return NAV_ITEMS.some((item) => item.key === key);
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
  const currentView = state.ui.view;

  return (
    <Layout style={{ height: "100vh", overflow: "hidden" }}>
      <Header
        style={{
          alignItems: "center",
          display: "flex",
          flex: "0 0 48px",
          height: 48,
          lineHeight: "48px",
          padding: 0,
        }}
      >
        <Menu
          items={NAV_ITEMS.map((item) => ({
            icon: item.icon,
            key: item.key,
            label: item.label,
          }))}
          mode="horizontal"
          onClick={({ key }) => {
            if (isViewType(key)) {
              dispatch({ type: "viewChanged", view: key });
            }
          }}
          selectedKeys={[currentView]}
          style={{ borderBottom: "none", flex: 1, minWidth: 0 }}
        />
        <AuthStatus />
      </Header>

      <Content style={{ flex: 1, minHeight: 0, overflow: "hidden", position: "relative" }}>
        <PersistentTab active={currentView === "home"}>
          <HomeBoard />
        </PersistentTab>
        <PersistentTab active={currentView === "demands"}>
          <DemandListPage />
        </PersistentTab>
        <PersistentTab active={currentView === "repos"}>
          <RepoListPage />
        </PersistentTab>
        <PersistentTab active={currentView === "settings"}>
          <SettingsPage />
        </PersistentTab>
      </Content>
    </Layout>
  );
}
