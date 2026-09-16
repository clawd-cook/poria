# Resource Layer — 实施计划

## 前置条件

- 依赖 `core-foundation`（已完成）：导入 `IResource`, `CapabilityMetadata`, `TIMEOUT`, `AgentTaskInput`, `AgentTaskResult` 等类型
- `packages/resources/` 目录已存在但为空
- 迁移源：`submodules/poria/packages/resource-terminal/src/index.ts`（shell exec 实现）

## 实施步骤

### Step 0: 包脚手架

- [ ] `packages/resources/package.json` — name: `@poria/resources`, type: module, 依赖 `@poria/core` (workspace:*)、`minimatch`、`nanoid`；devDeps: `typescript`, `vitest`
- [ ] `packages/resources/tsconfig.json` — extends 根 tsconfig.base.json
- [ ] `packages/resources/vitest.config.ts`
- [ ] `packages/resources/src/index.ts` — 空桶文件

验证: `pnpm install` + `pnpm --filter @poria/resources run typecheck` 通过。

### Step 1: Terminal 资源（迁移 + 增强）

**迁移自**: `submodules/poria/packages/resource-terminal/src/index.ts`

- [ ] `src/terminal/index.ts`:
  - 复制 `spawn` + timeout 逻辑（已有完整实现）
  - 改造接口：移除 `@dj-lib/poria-plugin-sdk` 依赖，改用 `IResource` 契约
  - 增强：`TimeoutError`（自定义 Error 子类，含 command + timeoutMs）
  - 接口：`exec(input: TerminalExecInput): Promise<TerminalExecResult>`
    - `TerminalExecInput`: `{ command: string, cwd?: string, env?: Record<string, string>, timeoutMs?: number }`
    - `TerminalExecResult`: `{ code: number, stdout: string, stderr: string }`
  - 超时档位常量已在 `@poria/core` 的 `TIMEOUT` 中定义，调用方按需传入

验证: typecheck 通过。

### Step 2: Worktree 资源（新建）

- [ ] `src/worktree/index.ts`:
  - `create(repo, pipelineId, baseBranch)` — 执行 `git worktree add` 在 `workspace/projects/{pipelineId}/{repo.name}` 下创建
  - `cleanDirtyState(repo, pipelineId)` — Recovery 用：`git checkout .` + `git clean -fd`
  - `remove(repo, pipelineId)` — `git worktree remove --force`，先 `existsSync` 检查路径
  - `path(repo, pipelineId)` — 返回 worktree 路径约定格式
  - 内部调用 Step 1 的 terminal exec
  - 实现 `IResource` 契约

验证: typecheck 通过。

### Step 3: Claude Agent 调度池（新建）

- [ ] `src/claude/agent-pool.ts`:
  - `initialize()` — 调用 Agent SDK `startup()`，保存 warm 实例
  - `dispatch(input: AgentTaskInput)` — 调用 `warm.query()`
    - AbortController 总超时（默认 30min）
    - 空闲超时检测：`setInterval` 每 30s 检查 `lastMessageAt`，超 5min → `controller.abort()`
    - `finally` 中清理 `clearTimeout(timer)` + `clearInterval(idleChecker)`
    - 遍历 `for await (msg of q)` 收集消息，每条 assistant 消息调 `onProgress` 回调
    - 返回 `AgentTaskResult` 含 sessionId、costUsd、messages
  - 各 Stage 调度参数表作为常量 `STAGE_AGENT_CONFIG`
  - **注意**：Agent SDK 是运行时依赖但可能未安装时需 graceful degrade。用 dynamic import + try/catch：SDK 不可用时 `initialize()` 抛明确错误

验证: typecheck 通过（SDK 类型通过 dynamic import 处理）。

### Step 4: Output Guard（新建）

- [ ] `src/claude/output-guard.ts`:
  - `check(agentOutput, config)` → `{ pass, violations }`
  - `OutputGuardConfig`: `{ allowedPaths: string[], maxDiffLines: number, blockedDependencies: string[] }`
  - `AgentOutput` 入参: `{ changedFiles: string[], totalDiffLines: number, addedDependencies: { name: string }[] }`
  - 三项检查逻辑：
    1. **文件范围**: `allowedPaths.length === 0` → warn（trdScope 空值 fallback）；否则逐文件 `minimatch` 检查，不匹配 → block
    2. **变更量**: `totalDiffLines > maxDiffLines` → warn
    3. **依赖安全**: 新增依赖名在 blockedDependencies 中 → block
  - `pass` = 无 block 级 violation

验证: typecheck 通过。

### Step 5: Session Tracker（新建）

- [ ] `src/claude/session-tracker.ts`:
  - `record(pipelineId, stageName, sessionId)` — 记录映射
  - `get(pipelineId, stageName)` → `sessionId | undefined`
  - P1 实现：内存 Map（实际持久化由 infra-persistence 的 stages 表 `agent_session_id` 列承担，session-tracker 是调用层的薄封装）

验证: typecheck 通过。

### Step 6: Tests + 最终验证

- [ ] `src/terminal/__tests__/terminal.test.ts`:
  - 正常命令返回 stdout/stderr + exit code
  - 超时 → kill + 抛 TimeoutError
  - cwd 生效
- [ ] `src/claude/__tests__/output-guard.test.ts`:
  - trdScope 为空 → warn，pass = true
  - 文件超出 trdScope → block，pass = false
  - diff 超限 → warn，pass = true
  - 被禁依赖 → block，pass = false
  - 混合场景：block + warn
- [ ] `src/worktree/__tests__/worktree.test.ts`:
  - path() 返回正确格式
  - create/remove 调用正确 git 命令（mock terminal）
  - cleanDirtyState 调用 git checkout + git clean
- [ ] `src/claude/__tests__/agent-pool.test.ts`:
  - 未初始化时 dispatch 抛错
  - dispatch 超时 → abort
  - 空闲超时 → abort
  - 返回 sessionId
- [ ] `src/claude/__tests__/session-tracker.test.ts`:
  - record + get round trip
  - 不存在的 key → undefined
- [ ] 入口 `src/index.ts` 统一 re-export
- [ ] `pnpm --filter @poria/resources run typecheck` 通过
- [ ] `pnpm --filter @poria/resources run test` 全部通过

## 验证命令

```bash
pnpm --filter @poria/resources run typecheck
pnpm --filter @poria/resources run test
```
