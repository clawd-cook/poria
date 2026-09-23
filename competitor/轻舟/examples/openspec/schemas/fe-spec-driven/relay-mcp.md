# Relay 远程 MCP 使用（`zero-design`）

> fe-spec-driven schema 内多个环节（relay-design-analysis / relay-component-analysis / workflow-implement，含其 `ui-static-integrate` 静态集成视觉校验）共用的 relay MCP 使用真源。用到 `get_design_*` / `get_screenshot` / `export_image` 等工具前先看这里。
>
> `lbcli relay get-token` 是工具层命令（怎么抓 token、写哪个配置文件），见 `lbcli/relay.md`；本文只讲 MCP 怎么用。

## 接入

本流程用 **Relay 远程 HTTP MCP**（`zero-design`）读取设计稿。

按下方「健康检查」先探活——
- 探活拿到响应 → 已配好，直接用；
- 探活发现工具不存在 / MCP 未配置 / 401 过期 → 跑 `lbcli relay get-token` 配置或刷新。

## URL 参数提取

```
https://relay.jd.com/file/design?id={fileKey}&page_id={pageId}&node_id={nodeId}
```

| 参数 | 用途 |
|------|------|
| `id` | fileKey，对应 MCP 工具 `designId` 参数（**每次调用必传**） |
| `page_id` | pageId |
| `node_id` | 具体节点（可选） |

- **URL 解码（必须）**：`page_id=271%3A1` 实为 `271:1`，`node_id=355%3A12730` 实为 `355:12730`；先 decode 再传。
- **分隔符规范化**：`36-62` → `36:62`。

## 健康检查与自愈

> **token 有 90 天有效期**（`get-token` 按 `numOfDays: 90` 申请/续期），写入 `~/.claude.json` 后一台机器管 90 天。到期后 MCP 调用返回 401，走下方自愈刷新即可——不是永久一次性配置。

**首次调用前**，先跑一次 `get_design_metadata`（传 fileKey + pageId）探活，按现象处置：

- ✅ **拿到响应** → 继续，进入工具速查。
- ❌ **工具不存在 / MCP server 未配置** →
  1. 打印一行「正在配置 Relay 远程 MCP，浏览器会打开 Relay 登录页（已登录则静默完成）」
  2. 用 Bash 跑 `lbcli relay get-token`
  3. 成功后提示用户「配置已写入 ~/.claude.json，请重启 Claude Code / reload MCP 后重试当前任务」
- ❌ **401 / token 过期** → 打印「Relay token 过期，刷新中」→ 跑 `lbcli relay get-token`（原地覆盖）→ 提示重启。
- ❌ **未登录 Relay**（`lbcli` 抛 AuthRequiredError） → 提示用户在打开的浏览器窗口完成 Relay 登录 → 登录后 agent 自己再跑一次 `lbcli relay get-token`，不用用户敲命令。
- ❌ **网络不通 / MCP 不可达** → 打印明确失败原因。

## 工具速查

**每个工具都必须传 `designId`（即 URL 的 fileKey）和 `nodeId`。**

| 工具 | 用途 | 关键参数 |
|------|------|---------|
| `get_design_metadata` | 节点结构 / 坐标 / children | `designId`, `nodeId`（必传） |
| `get_node_data` | 单节点原始 layerData（含 Auto Layout / fills / 尺寸）；供 `relay-schema-gen.py normalize` 生成静态稿 schema | `designId`, `nodeId`, `includeChildrenData`（取整棵子树置 true） |
| `get_screenshot` | 帧截图（视觉确认） | `designId`, `nodeId`；URL 短有效期 |
| `get_design_context` | 单节点参考代码 + 截图 | `designId`, `nodeId`；适合深入分析单组件 |
| `get_variables` | 设计令牌（颜色/字号/间距） | `designId`, `nodeId` |
| `export_image` | 导出节点为 CDN 图片（7 天有效） | `designId`, `nodeId`, `scale`, `format` |
| `export_svg` | 导出节点为 SVG（7 天有效） | `designId`, `nodeId` |

## 常见陷阱

| 陷阱 | 规避 |
|------|------|
| 忘记传 `designId` | 每次工具调用必传；从 URL 的 `id=` 参数取 |
| metadata 超大（200K+） | 只关注 children 第一层，按需深入 |
| 帧名是自动命名（"Frame 123"） | 看帧内 TEXT 节点或截图识别 |
| 截图 URL 过期 | 拿到立即使用，超时重新 `get_screenshot` |
| page_id / node_id 含 URL 编码 | `%3A` → `:`，decode 后再传 |
| nodeId 用 `-` 分隔 | 替换为 `:` 再传 |
