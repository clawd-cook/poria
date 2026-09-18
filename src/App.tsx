import { App as AntApp, ConfigProvider } from "antd";
import zhCN from "antd/locale/zh_CN";

import { Shell } from "./components/Shell";
import { StoreProvider } from "./state/store";

export default function App() {
  return (
    <ConfigProvider locale={zhCN}>
      <AntApp>
        <StoreProvider>
          <Shell />
        </StoreProvider>
      </AntApp>
    </ConfigProvider>
  );
}
