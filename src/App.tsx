import { App as AntApp, ConfigProvider } from "antd";
import zhCN from "antd/locale/zh_CN";

import { Shell } from "./components/Shell";
import { StoreProvider } from "./state/store";
import { poriaTheme } from "./theme";

export default function App() {
  return (
    <ConfigProvider locale={zhCN} theme={poriaTheme}>
      <AntApp>
        <StoreProvider>
          <Shell />
        </StoreProvider>
      </AntApp>
    </ConfigProvider>
  );
}
