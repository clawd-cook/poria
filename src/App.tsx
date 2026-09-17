import { App as AntApp, ConfigProvider, theme } from "antd";
import zhCN from "antd/locale/zh_CN";

import { Shell } from "./components/Shell";
import { StoreProvider } from "./state/store";

export default function App() {
  return (
    <ConfigProvider
      locale={zhCN}
      theme={{
        algorithm: theme.darkAlgorithm,
        token: {
          colorPrimary: "#1677ff",
          borderRadius: 6,
        },
      }}
    >
      <AntApp>
        <StoreProvider>
          <Shell />
        </StoreProvider>
      </AntApp>
    </ConfigProvider>
  );
}
