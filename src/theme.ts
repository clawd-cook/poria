import type { ThemeConfig } from "antd";

/**
 * Ant Design v6 light seeds + Swiss / Minimal chrome.
 * Keep a single primary; do not mint extra brand accents.
 */
export const poriaTheme: ThemeConfig = {
  components: {
    Button: {
      defaultShadow: "none",
      primaryShadow: "none",
    },
    Card: {
      boxShadowTertiary: "none",
    },
    Layout: {
      bodyBg: "#F5F5F5",
      siderBg: "#FFFFFF",
    },
    Menu: {
      itemBorderRadius: 4,
      itemMarginBlock: 4,
      itemMarginInline: 8,
      itemSelectedBg: "#E6F4FF",
    },
    Table: {
      headerBg: "#F5F5F5",
    },
    Tag: {
      borderRadiusSM: 4,
    },
  },
  token: {
    borderRadius: 4,
    borderRadiusLG: 4,
    boxShadow: "none",
    boxShadowSecondary: "none",
    boxShadowTertiary: "none",
    colorBgContainer: "#FFFFFF",
    colorBgLayout: "#F5F5F5",
    colorPrimary: "#1677FF",
    fontFamily:
      "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, 'Noto Sans', sans-serif",
    fontSize: 14,
    fontWeightStrong: 600,
    motionDurationFast: "0.1s",
    motionDurationMid: "0.2s",
    motionDurationSlow: "0.3s",
  },
};
