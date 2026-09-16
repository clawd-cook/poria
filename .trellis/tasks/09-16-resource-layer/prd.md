# Resource Layer: Claude Agent + OutputGuard + Terminal + Worktree — PRD

## Goal

实现 `packages/resources/` 层：本机资源抽象，包括 Claude Code Agent SDK 调度、Agent 输出拦截、Shell 执行（从 submodules 迁移）、Git worktree 生命周期管理。所有资源实现 `IResource` 契约（来自 core-foundation）。

---

## 依赖

- **core-foundation**：导入 `IResource` 契约接口、共享类型定义

---

## 核心需求

### R1: Claude Agent 调度池（agent-pool.ts）

**包路径**: `packages/resources/claude/agent-pool.ts`

- **Warm Start**：Worker 进程启动时调用 `startup()` 一次，加载 Agent SDK 二进制，后续 dispatch 复用
- **Dispatch**：接受 `AgentTaskInput`（prompt, worktreePath, systemPrompt, model, maxBudgetUsd, maxTurns, timeoutMs, extraTools, onProgress），返回 `AgentTaskResult`
- **安全配置**：permissionMode = "auto", permissionPrompts = "none"（永不阻塞等待人工确认）
- **双重超时**：
  - 总超时：默认 30min（`timeoutMs`），通过 AbortController 中止
  - 空闲超时：连续 5min 无消息输出 → kill agent（Design F-08）
  - 空闲检测 interval 必须在 finally 中清理（Design §16 F-08）
- **进度回调**：每条 assistant 消息通过 `onProgress` 回调上报，供 Pipeline 事件日志记录
- **Session 持久化**：dispatch 返回 `sessionId`，用于断点续跑
- **各 Stage 调度参数**：

| Stage | allowedTools | maxBudgetUsd | maxTurns | timeoutMs |
|-------|-------------|-------------|----------|-----------|
| review_prd | Read, Grep | $1 | 10 | 5min |
| design | Read, Edit, Grep | $3 | 20 | 10min |
| dev | Read, Edit, Bash, Glob, Grep | $10 | 100 | 30min |
| cr | Read, Bash, Grep | $5 | 30 | 15min |

### R2: Agent 输出拦截（output-guard.ts）

**包路径**: `packages/resources/claude/output-guard.ts`

- **文件范围检查**：Agent 修改的文件必须匹配 `trdScope`（glob 模式数组，由 design stage 从 TRD.md 提取）
  - 使用 `minimatch` 匹配
  - **trdScope 空值 fallback**：trdScope 为空/undefined 时**跳过文件范围检查**（severity: "warn"），不阻断 Pipeline。空 trdScope 更可能是 TRD 提取失败，而非"禁止修改所有文件"
  - 超出范围的文件 → severity: "block"
- **变更量检查**：`totalDiffLines > maxDiffLines`（默认 500）→ severity: "warn"（不阻断，标记需人工 review）
- **依赖安全检查**：新增依赖名在 `blockedDependencies` 黑名单中 → severity: "block"
- **返回值**：`{ pass: boolean, violations: Violation[] }`，pass = 无 block 级 violation

### R3: Session Tracker（session-tracker.ts）

**包路径**: `packages/resources/claude/session-tracker.ts`

- 记录每次 Agent dispatch 的 `sessionId`，关联到 `pipelineId + stageName`
- 存储在 SQLite stages 表的 `agent_session_id` 列（infra-persistence 提供）
- 中断恢复时可通过 `sessionId` resume Agent session

### R4: Terminal 执行（terminal/）

**包路径**: `packages/resources/terminal/`

- **迁移来源**：`submodules/poria/packages/resource-terminal/`
- **Shell exec**：spawn 子进程执行命令，捕获 stdout/stderr
- **超时档位**（Design A4）：

| 档位 | 超时 | 用途 |
|------|------|------|
| GIT_SHORT | 30,000ms | git status, git branch |
| GIT_MEDIUM | 120,000ms | git push, git revert, git merge |
| GIT_LONG | 300,000ms | git clone, 大仓 git push |
| BUILD | 600,000ms | npm run build |
| AGENT | 1,800,000ms | Claude Code agent 执行 |

- **超时处理**：超时后 kill 子进程 + 抛出 `TimeoutError`
- **cwd 支持**：每次 exec 可指定工作目录
- **实现 IResource 契约**

### R5: Git Worktree 生命周期（worktree/）

**包路径**: `packages/resources/worktree/`

- **create**：在指定 repo 下创建 worktree（`git worktree add`），返回 worktree 路径
- **cleanDirtyState**：Recovery 场景下清理未提交变更（`git checkout .` + `git clean -fd`），用于中断恢复（Design §4.2 F7）
- **remove**：删除 worktree（`git worktree remove --force`），先检查路径是否存在
- **path(repo, pipelineId)**：返回 worktree 路径的约定格式
- **实现 IResource 契约**

---

## 约束

- Node.js v24.20.0 + TypeScript
- pnpm 11.23.0 monorepo workspace
- Agent SDK 依赖：`@anthropic-ai/claude-agent-sdk`
- minimatch 用于 glob 匹配（OutputGuard）
- 不引入其他外部运行时依赖

---

## 非目标

- Agent resume 的完整实现（P1 只记录 sessionId，resume 降级为重跑）
- Agent 并发调度（MVP 单 Agent 串行）
- Terminal 远程执行（仅本机 shell）

---

## Acceptance Criteria

- [ ] OutputGuard unit test：trdScope 为空 → warn 而非 block，允许所有变更
- [ ] OutputGuard unit test：文件超出 trdScope → block
- [ ] OutputGuard unit test：diff 行数超限 → warn
- [ ] OutputGuard unit test：新增被禁依赖 → block
- [ ] Terminal exec：指定超时后超时 → 进程被 kill + 抛出 TimeoutError
- [ ] Terminal exec：正常命令在超时内完成 → 返回 stdout/stderr
- [ ] Agent pool：初始化（mock SDK startup）成功
- [ ] Agent pool：dispatch 超时 → AbortController 中止 + 返回 error
- [ ] Agent pool：空闲超时 5min 无消息 → agent 被 kill
- [ ] Agent pool：dispatch 返回 sessionId
- [ ] Worktree：create → 目录存在 + git worktree list 可见
- [ ] Worktree：cleanDirtyState → 未提交变更被清理
- [ ] Worktree：remove → 目录不存在 + git worktree list 不可见
- [ ] Session tracker：记录 sessionId → 可按 pipelineId + stage 查回
- [ ] 全部 resource 实现 IResource 契约接口
- [ ] TypeScript 编译通过，无类型错误
