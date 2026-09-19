# Visual Theme (Tailwind + shadcn, restaurant-warm)

> Light theme for the Poria desktop shell. Captured 2026-09-19 from ui-ux-pro-max (`design-system/poria/MASTER.md`).

## 1. Scope / Trigger

Use this spec when changing `src/styles.css` tokens, `PageFrame`, sider chrome, shadcn primitives, or page-level padding/title. Do **not** load Google Fonts from a CDN, reintroduce Ant Design, or restyle Vite `:1420` as proof — verify in the Poria window.

## 2. Signatures

- Tokens live in `src/styles.css` `:root` + `@theme inline`; Tailwind utilities map to them (`bg-primary`, `text-muted-foreground`)
- `src/styles.css` hides all scrollbars (`scrollbar-width: none` / `::-webkit-scrollbar { display: none }`); overflow still scrolls
- Seeds: primary `#DC2626` (CTAs only), accent `#A16207`, background `#FAFAF8`, sidebar `#EBE8E3`, foreground `#1C1917`, card `#FFFFFF`, border `#E7E3DC`, radius `0.5rem`
- Fonts (self-hosted `@fontsource`): headings Playfair Display SC (`font-serif` / `.font-display`); body Karla (`font-sans`)
- Motion: 150–300ms color/opacity; honor `prefers-reduced-motion`
- Hierarchy: hairline `border-border` on cream layout; cards are white plates
- Nav selected: `bg-primary/10 text-primary` (not a solid red fill)
- Pages wrap content in `PageFrame` (`h1` serif). Nested sections use `PageSectionTitle` (`h2`)
- Primitives: `src/components/ui/*` (Button, Card, Dialog, Sheet, Input, Select, Tabs, Alert, Badge, Table)
- Icons: Lucide outline, `size-4` in controls; decorative icons `aria-hidden`
- Toasts: `sonner` (`toast.success` / `toast.error` / `toast.warning` / `toast.info`)

## 3. Contracts

| Surface | Owns |
| --- | --- |
| Theme tokens | Semantic CSS variables. Status uses `success` / `warning` / `destructive` / `primary`. No second brand palette. |
| Layout | `bg-background` canvas. Sider is `bg-sidebar`, not `bg-card`. Cards stay white. |
| PageFrame | Title + optional description/extra; padding `p-6`. |
| Cards / repo rows | Hairline border, no drop shadow. Clickable cards use `hover:border-primary`. |
| Scrollbars | Hidden everywhere; never draw a gutter or thumb. 看板 lane: vertical wheel pans horizontally unless the hovered column can still scroll vertically; empty chrome can be dragged. |
| Icons | `lucide-react`. Color from tokens (`text-primary`, `text-success`), never raw hex. |

## 4. Validation & Error Matrix

| Condition | What you see |
| --- | --- |
| Hard-coded `#1677ff` / Ant Design `theme.useToken()` | Breaks the restaurant-warm system — use Tailwind tokens |
| Two primary buttons on one decision | Demote extras to `variant="outline"` |
| `prefers-reduced-motion: reduce` | Transitions collapse via `src/styles.css` |
| Icon-only control without name | Add `aria-label` (or visible text) |

## 5. Good / Base / Bad Cases

- **Good**: Sider is warm stone `#EBE8E3`, not white; content sits on light canvas `#FAFAF8`; cards are white; selected nav is a light terracotta wash, not a red slab.
- **Base**: Tailwind v4 + shadcn New York, light only, zh-CN copy.
- **Bad**: Ant Design ConfigProvider; Inter/Helvetica-only chrome; Swiss `#1677FF`; Google Fonts CDN; emoji as icons.

## 6. Tests Required

- `pnpm typecheck`
- Manual: `cargo tauri dev`, Poria window — 看板滚轮横移列、列内仍上下滚、拖空白处平移；技能抽屉/仓库无条可滚

## 7. Wrong vs Correct

#### Wrong

```text
import { Button } from "antd"
theme.useToken()
icon style={{ color: "#1677ff" }}
Page Title with inline padding 24 copied per page
```

#### Correct

```text
import { Button } from "@/components/ui/button"
className="text-primary"
<PageFrame title="技能">…</PageFrame>
`src/styles.css` hides scrollbars globally
```
