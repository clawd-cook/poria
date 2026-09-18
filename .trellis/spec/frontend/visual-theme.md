# Visual Theme (Ant Design + Swiss / Minimal)

> Light theme for the Poria desktop shell. Captured 2026-09-18.

## 1. Scope / Trigger

Use this spec when changing `ConfigProvider` theme, `src/theme.ts`, `PageFrame`, sider chrome, or page-level padding/title. Do **not** invent a second palette, load web fonts, or restyle Vite `:1420` as proof — verify in the Poria window.

## 2. Signatures

- `src/theme.ts` → `poriaTheme: ThemeConfig` passed to `ConfigProvider` in `src/App.tsx`
- Seeds: `colorPrimary` `#1677FF`, `colorBgLayout` `#F5F5F5`, `colorBgContainer` `#FFFFFF`, `borderRadius` `4`, `fontSize` `14`, `fontWeightStrong` `600`
- Font stack: OS UI fonts including `Helvetica Neue` (no Google Fonts)
- Motion: `motionDurationFast/Mid/Slow` = `0.1s` / `0.2s` / `0.3s`
- Shadows: `boxShadow*` tokens are `none`; hierarchy is border + layout background
- Menu selected: `itemSelectedBg` `#E6F4FF`
- Pages wrap content in `PageFrame` (`h1` at heading-3 size). Nested sections use `PageSectionTitle` (`h2` at heading-5 size).

## 3. Contracts

| Surface | Owns |
| --- | --- |
| Theme tokens | Single primary. Status uses `success` / `warning` / `error` / `info`. Preset hues only on Tag/chart. |
| Layout | `bg-layout` around `bg-container`. Sider is container + 1px `colorBorderSecondary`. |
| PageFrame | Title + optional description/extra; padding `token.paddingLG`. |
| Cards / repo rows | Hairline border, no drop shadow. Clickable cards keep `hoverable`. |
| Icons | `@ant-design/icons`. Color from tokens (`colorPrimary`, `colorSuccess`), never raw hex. |

## 4. Validation & Error Matrix

| Condition | What you see |
| --- | --- |
| Hard-coded `#1677ff` / `#52c41a` / `#303030` | Breaks theme and light/dark later — use `theme.useToken()` |
| Two `type="primary"` buttons on one decision | Demote extras to default |
| `prefers-reduced-motion: reduce` | Transitions collapse via `src/styles.css` |

## 5. Good / Base / Bad Cases

- **Good**: Sider wordmark is tracked uppercase; content sits on `#F5F5F5`; cards are bordered white; selected nav is `#E6F4FF`.
- **Base**: Ant Design v6 default algorithm, `zh_CN`, light only.
- **Bad**: Teal/orange brand overlay; Inter from Google Fonts; card shadows; magic padding like `11px`.

## 6. Tests Required

- `pnpm typecheck`
- Manual: `cargo tauri dev`, Poria window — sider, 看板, 需求, 技能, 仓库, 设置

## 7. Wrong vs Correct

#### Wrong

```text
ConfigProvider with no theme
Skill icon color="#1677ff"
Repo row border #303030
Page Title level={4} + padding 24 copied per page
```

#### Correct

```text
ConfigProvider theme={poriaTheme}
icon style={{ color: token.colorPrimary }}
border: token.colorBorderSecondary
<PageFrame title="技能">…</PageFrame>
```
