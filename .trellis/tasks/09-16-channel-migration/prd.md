# Channel Migration — PRD v1.0

## Goal

将 `submodules/poria/packages/` 中的三个 channel 包（channel-xingyun、channel-joyspace、channel-coding）**源码迁移**到主仓 `packages/channels/`，重构为统一的 IChannel 契约实现；同时新建 `jme/` 京ME 通道（JoyClaw 桥接）。迁移完成后主仓对 `@dj-lib/poria-*` 外部包的依赖归零。

---

## 依赖

- **前置**: `core-foundation`（提供 IChannel 契约接口、共享类型）

---

## 核心需求

### M1: xingyun — 星云需求/卡片/PRD 集成

**迁移来源**: `submodules/poria/packages/channel-xingyun/src/`

**目标路径**: `packages/channels/xingyun/`

**迁移文件清单**:

| 源文件 | 迁移内容 | 备注 |
|--------|----------|------|
| `demandUrl.ts` | `parseXingyunDemandUrl()` — 链接解析 | 白名单域名 `*.xingyun.jd.com`，提取 demandId + demandCode |
| `jacp/client.ts` | JACP HTTP 客户端基础设施 | cookie 注入、错误处理 |
| `jacp/demands.ts` | `getDemandById()` — GET /openapi/v3/demands/{demandId} | 返回 DemandDetail |
| `jacp/cards.ts` | `listCardAttachments()` — GET /openapi/v3/cards/code/{demandCode} | 返回 attachments[] |
| `jacp/prdAttachmentLink.ts` | `isJoySpacePrdLink()` — PRD 链接识别 | JoySpace URL 匹配 |
| `jacp/easyci.ts` | `bindBranch()` — 分支绑定 | 返回 { branch, changeId, baseBranch } |
| `jacp/spaces.ts` | 空间相关 API | 按需迁移 |
| `prd.ts` | `resolvePrdFromAttachments()` — PRD 解析策略 | 单个/多个/歧义/零个 |
| `git.ts` | `featureBranchName()`, `featureSlug()` — 分支名生成 | 直接复用 |
| `fixture.ts` | Fixture 模式 mock 数据 | PORIA_XINGYUN_FIXTURE=1 |
| `index.ts` | 模块导出 | 重构为 IChannel 实现 |

**新增**:
- `demand-guard.ts` — 需求状态判定（Design A10），MVP 阶段不做状态过滤，预留接口

### M2: joyspace — JoySpace 文档导出

**迁移来源**: `submodules/poria/packages/channel-joyspace/src/`

**目标路径**: `packages/channels/joyspace/`

**迁移文件清单**:

| 源文件 | 迁移内容 | 备注 |
|--------|----------|------|
| `export.ts` | `exportToMarkdown()` — JoySpace → Markdown 导出 | 内部处理图片/表格/Mermaid |
| `vendor/` | 第三方依赖封装 | 按需迁移 |
| `index.ts` | 模块导出 | 重构为 IChannel 实现 |

### M3: coding — EasyCI 仓库/分支/MR 操作

**迁移来源**: `submodules/poria/packages/channel-coding/src/`

**目标路径**: `packages/channels/coding/`

**迁移文件清单**:

| 源文件 | 迁移内容 | 备注 |
|--------|----------|------|
| `easyci.ts` | EasyCI HTTP 客户端 | 仓库/分支/CI 操作 |
| `mergeRequest.ts` | `createMergeRequest()`, `projectIdFromGitUrl()` | MR 创建 + GitLab project path 推导 |
| `gitUrl.ts` | Git URL 解析工具 | |
| `index.ts` | 模块导出 | 重构为 IChannel 实现 |

**新增（Design A11 + F-03）**:

| 新增方法 | 用途 | 底层 API |
|----------|------|----------|
| `getMrStatus(projectPath, iid)` | MR 合并状态轮询（P1 必须） | GET /api/v4/projects/{id}/merge_requests/{iid} |
| `findMr(query)` | 幂等 MR 创建 — 先查已有 open MR（P1 必须） | GET /api/v4/projects/{id}/merge_requests?source_branch=...&state=opened |

### M4: jme — 京ME 消息通道（新建）

**目标路径**: `packages/channels/jme/`

**交付物（Design A9）**:

| 文件 | 内容 | 备注 |
|------|------|------|
| `joyclaw-bridge.ts` | JoyClaw Agent 桥接 | spawn node openclaw.mjs，解析 stdout JSON |
| `index.ts` | `send()` — 发送京ME消息 | 自然语言指令，由 JoyClaw agent 完成发送 |
| | `readReplies()` — 读取回复 | 轮询式，调 JoyClaw "查看最近消息" |
| | `ensureGatewayAlive()` — 健康检查 | 检查 :18810 端口 / gateway 进程 |

**设计约束（Design A9）**:
1. 消息格式是纯文本（JoyClaw agent 自行决定京ME操作方式）
2. 依赖 JoyClaw gateway 常驻运行（:18810）
3. 回复监听是轮询，没有 webhook
4. 回复解析放宽：模糊匹配（包含"修复"/"fix" → resume；包含"跳过"/"skip" → skip；包含"取消"/"cancel" → cancel）

### M5: defect — 缺陷管理（预留）

**目标路径**: `packages/channels/defect/`

- P1 不实现，仅创建目录 + 空 `index.ts` 导出占位

---

## 约束

### C1: 迁移方式

- **源码复制 + 重构**，非 npm install
- submodules/poria/ 作为**只读参考**，不作为运行时依赖
- 迁移后主仓不再有任何 `@dj-lib/poria-*` 的 import

### C2: 契约一致性

- 所有 channel 实现 `IChannel` 接口（来自 `core-foundation` 的 `core/contracts/`）
- fixture 模式通过环境变量控制：`PORIA_XINGYUN_FIXTURE=1`、`PORIA_JOYSPACE_FIXTURE=1`、`PORIA_CODING_FIXTURE=1`、`PORIA_JME_FIXTURE=1`

### C3: 技术栈

- Node.js v24.20.0 + TypeScript
- pnpm 11.23.0 monorepo
- 不引入新的外部 HTTP 客户端库（复用已有 fetch / JACP client 封装）

---

## 非目标

- 不重写已有 channel 的核心逻辑（迁移为主，仅做接口适配）
- 不实现 defect channel 功能
- 不做 channel 的 E2E 真实 API 测试（P1 仅 fixture 模式）
- 不实现 JoyClaw gateway 的自动启动/安装

---

## Acceptance Criteria

- [ ] `packages/channels/xingyun/` 迁移完成，`parseXingyunDemandUrl` 正确从真实链接格式 `http://xingyun.jd.com/demands/view/{code}/-1?demandId={id}` 提取 demandId + demandCode
- [ ] `packages/channels/joyspace/` 迁移完成，`exportToMarkdown` 在 fixture 模式下可正常调用
- [ ] `packages/channels/coding/` 迁移完成，新增 `getMrStatus` 返回正确的状态枚举，`findMr` 可查找已有 open MR
- [ ] `packages/channels/jme/` 新建完成，JoyClaw bridge 在 fixture 模式下可发送消息和读取回复
- [ ] 所有 channel 实现 `IChannel` 契约接口
- [ ] 所有 channel 在 fixture 模式下（`PORIA_*_FIXTURE=1`）unit test 通过
- [ ] 主仓代码中无 `@dj-lib/poria-*` 的 import 引用
- [ ] TypeScript 编译通过（`pnpm -r --filter ./packages/channels run typecheck`）
