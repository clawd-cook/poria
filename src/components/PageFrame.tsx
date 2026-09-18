import { Flex, Typography, theme } from "antd";
import type { ReactNode } from "react";

const { Text, Title } = Typography;

export function PageFrame({
  children,
  description,
  extra,
  title,
}: {
  children?: ReactNode;
  description?: ReactNode;
  extra?: ReactNode;
  title: string;
}) {
  const { token } = theme.useToken();

  return (
    <div
      style={{
        background: token.colorBgLayout,
        minHeight: "100%",
        padding: token.paddingLG,
      }}
    >
      <Flex
        align="flex-start"
        gap={token.marginMD}
        justify="space-between"
        style={{ marginBottom: token.marginLG }}
      >
        <div>
          <Title
            level={1}
            style={{
              fontSize: token.fontSizeHeading3,
              fontWeight: token.fontWeightStrong,
              letterSpacing: "-0.02em",
              lineHeight: token.lineHeightHeading3,
              margin: 0,
            }}
          >
            {title}
          </Title>
          {description ? (
            <Text type="secondary" style={{ display: "block", marginTop: token.marginXS }}>
              {description}
            </Text>
          ) : null}
        </div>
        {extra ? <div>{extra}</div> : null}
      </Flex>
      {children}
    </div>
  );
}

export function PageSectionTitle({ children }: { children: ReactNode }) {
  const { token } = theme.useToken();
  return (
    <Title
      level={2}
      style={{
        fontSize: token.fontSizeHeading5,
        fontWeight: token.fontWeightStrong,
        marginBottom: token.marginSM,
        marginTop: 0,
      }}
    >
      {children}
    </Title>
  );
}
