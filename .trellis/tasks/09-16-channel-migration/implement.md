# Channel Migration — 实施计划

## 前置条件

- `core-foundation` 已完成（提供 IChannel 契约接口 + 共享类型）
- `submodules/poria/packages/` 三个 channel 包可作只读参考
- `packages/channels/` 目录已存在但为空

## 源码概况

| 包               | 文件数                           | 总行数   | 外部依赖                                                   |
| ---------------- | -------------------------------- | -------- | ---------------------------------------------------------- |
| channel-xingyun  | 11 (.ts) + 1 (fixture)           | ~1318 行 | `@dj-lib/poria-plugin-sdk`, `@dj-lib/poria-channel-coding` |
| channel-joyspace | 2 (.ts) + 6 (vendor .mjs/.d.mts) | ~411 行  | `@dj-lib/poria-plugin-sdk`                                 |
| channel-coding   | 4 (.ts)                          | ~503 行  | `@dj-lib/poria-plugin-sdk`                                 |

所有 `@dj-lib/poria-plugin-sdk` 引用需替换为 `@poria/core` 的 IChannel 契约；`channel-xingyun` 对 `channel-coding` 的依赖需改为相对导入。

## 实施步骤

### Step 0: 包脚手架

每个 channel 独立包，统一工作区配置。

- [ ] 更新 `pnpm-workspace.yaml` — 增加 `packages/channels/*`
- [ ] 创建 `packages/channels/xingyun/package.json` — name: `@poria/channel-xingyun`, deps: `@poria/core`
- [ ] 创建 `packages/channels/joyspace/package.json` — name: `@poria/channel-joyspace`, deps: `@poria/core`
- [ ] 创建 `packages/channels/coding/package.json` — name: `@poria/channel-coding`, deps: `@poria/core`
- [ ] 创建 `packages/channels/jme/package.json` — name: `@poria/channel-jme`, deps: `@poria/core`
- [ ] 每个包: `tsconfig.json` (extends root), `vitest.config.ts`, `src/index.ts` (空桶)
- [ ] `pnpm install` 验证工作区解析

验证: `pnpm -r --filter './packages/channels/*' run typecheck` 通过。

### Step 1: Xingyun channel 迁移

**复制 + 适配** 共 12 个文件（~1318 行）。

- [ ] `src/jacp/client.ts` — 复制 JACP HTTP 客户端，移除 `@dj-lib/poria-plugin-sdk` import，类型改用 `@poria/core`
- [ ] `src/jacp/demands.ts` — 复制 getDemandById，返回类型适配 DemandMetadata
- [ ] `src/jacp/cards.ts` — 复制 listCardAttachments，返回类型适配 CardAttachment
- [ ] `src/jacp/prdAttachmentLink.ts` — 复制 isJoySpacePrdLink，纯逻辑无需改动
- [ ] `src/jacp/easyci.ts` — 复制 bindBranch，移除对 `@dj-lib/poria-channel-coding` 的依赖（改为相对导入或内联）
- [ ] `src/jacp/spaces.ts` — 复制（按需，28 行）
- [ ] `src/demandUrl.ts` — 复制 parseXingyunDemandUrl，纯逻辑
- [ ] `src/prd.ts` — 复制 resolvePrdFromAttachments，纯逻辑
- [ ] `src/git.ts` — 复制 featureBranchName/featureSlug，纯逻辑
- [ ] `src/fixture.ts` — 复制 fixture mock 数据
- [ ] `src/demand-guard.ts` — **新建**，isDemandActive() MVP 空实现（Design A10）
- [ ] `src/index.ts` — 重写为 IChannel 实现，统一导出

关键适配点:

- `@dj-lib/poria-plugin-sdk` → `@poria/core` (IChannel, CapabilityMetadata 等)
- `@dj-lib/poria-channel-coding` → 直接在 xingyun 包内引用 easyci 函数，或改为从 `@poria/channel-coding` 导入
- credentials 类型 → 从 `@poria/core` ChannelContext 获取

验证: typecheck 通过。

### Step 2: Joyspace channel 迁移

**复制 + 适配** 共 8 个文件（~411 行 .ts + 6 个 vendor 文件）。

- [ ] `src/vendor/` — 复制全部 6 个 .mjs/.d.mts 文件（第三方封装，不修改）
- [ ] `src/export.ts` — 复制 exportToMarkdown，移除 `@dj-lib/poria-plugin-sdk` import
- [ ] `src/index.ts` — 重写为 IChannel 实现

关键适配点:

- vendor 文件是 .mjs（已编译），直接复制，tsconfig 中配置 `allowJs: true`
- export.ts 的内部逻辑不改，仅适配接口签名

验证: typecheck 通过。

### Step 3: Coding channel 迁移 + 新增 API

**复制 + 适配** 4 个文件（~503 行）+ **新写** 2 个方法。

- [ ] `src/easyci.ts` — 复制 EasyCI HTTP 客户端
- [ ] `src/mergeRequest.ts` — 复制 createMergeRequest + projectIdFromGitUrl
- [ ] `src/gitUrl.ts` — 复制 Git URL 解析工具
- [ ] `src/index.ts` — 重写为 IChannel 实现

**新增（Design A11 + F-03）**:

- [ ] `src/mergeRequest.ts` 中新增 `getMrStatus(projectPath: string, iid: number): Promise<MrStatus>`
  - 底层: GET /api/v4/projects/{encoded_path}/merge_requests/{iid}
  - 返回: `"opened" | "closed" | "merged" | "locked"`
- [ ] `src/mergeRequest.ts` 中新增 `findMr(query: FindMrQuery): Promise<MrInfo | null>`
  - 底层: GET /api/v4/projects/{id}/merge_requests?source_branch=...&target_branch=...&state=opened
  - 返回第一条匹配记录或 null
- [ ] `src/fixture.ts` — 新建 fixture mock 覆盖 getMrStatus + findMr

验证: typecheck 通过。

### Step 4: JME channel 新建

**全部新写**（Design A9）。

- [ ] `src/joyclaw-bridge.ts` — JoyClaw Agent 桥接核心
  - `runAgent(message, timeoutSec)`: spawn node openclaw.mjs，解析 stdout JSON
  - 配置路径: `~/.joyclaw/node/node-v22.16.0-darwin-arm64/bin/node`, `~/.joyclaw/apps/v2.5.6/openclaw.mjs`
  - 超时处理 + 进程清理
- [ ] `src/index.ts` — IChannel 实现
  - `send(target, message)`: 通过 runAgent 发送京ME消息
  - `readReplies(chatName, since)`: 通过 runAgent 查看聊天消息
  - `ensureGatewayAlive()`: 检查 :18810 端口
  - `parseReply(text)`: 模糊匹配回复关键字（修复/fix→resume, 跳过/skip→skip, 取消/cancel→cancel）
- [ ] `src/fixture.ts` — fixture 模式（PORIA_JME_FIXTURE=1），mock send/readReplies/ensureGatewayAlive

验证: typecheck 通过。

### Step 5: Defect channel 预留

- [ ] `packages/channels/defect/package.json` — 最小包配置
- [ ] `packages/channels/defect/tsconfig.json`
- [ ] `packages/channels/defect/src/index.ts` — 空导出占位

验证: typecheck 通过。

### Step 6: Fixture 模式验证

- [ ] 为每个 channel 编写 fixture 模式 unit test:
  - `xingyun/__tests__/demandUrl.test.ts` — parseXingyunDemandUrl 链接解析
  - `xingyun/__tests__/prd.test.ts` — resolvePrdFromAttachments 策略
  - `xingyun/__tests__/git.test.ts` — featureBranchName/featureSlug
  - `coding/__tests__/mergeRequest.test.ts` — getMrStatus fixture + findMr fixture
  - `coding/__tests__/gitUrl.test.ts` — projectIdFromGitUrl
  - `jme/__tests__/parseReply.test.ts` — 模糊匹配回复解析
  - `jme/__tests__/fixture.test.ts` — fixture 模式 send/readReplies

验证: `pnpm -r --filter './packages/channels/*' run test` 全通过。

### Step 7: 最终验证

- [ ] `pnpm -r --filter './packages/channels/*' run typecheck` 通过
- [ ] `pnpm -r --filter './packages/channels/*' run test` 全通过
- [ ] `grep -r "@dj-lib/poria" packages/` 返回空（无残留外部包引用）
- [ ] 所有 channel 的 index.ts 导出 IChannel 实现

## 验证命令

```bash
pnpm -r --filter './packages/channels/*' run typecheck
pnpm -r --filter './packages/channels/*' run test
grep -r "@dj-lib/poria" packages/
```
