# Poria：把 AI 交付流水线搬回本地的工程实践

团队之前的 AI 编码工具跑在远程 Linux 机器上，两个问题一直没解决：一是远程机器的算力不如手边的 M2 Pro，编译和 agent 调用都慢；二是代码改了什么，在远程环境里看不方便。Poria 把整条交付流水线搬到 macOS 桌面端，从需求导出到提交合并请求，全部在本地完成。下面展开讲 Tauri + Rust 的技术选型、七阶段流水线的编排方式、状态机和门禁的设计，以及这套方案的边界。

## 为什么选本地

两个直接原因。

**算力倒挂。** 一台 M2 Pro MacBook Pro 的单核性能和内存带宽，比团队分配到的远程 Linux 开发机好。Rust 编译、TypeScript 类型检查、Claude CLI 的 token 处理，都吃单核和内存。任务丢到远程反而更慢，还要排队等资源。

**变更不可见。** AI agent 在远程改代码，开发者要 SSH 进去或者拉 diff 才能看到改了什么。本地桌面端直接用 git worktree，改完的代码就在文件系统里，IDE 打开就能看，diff 工具随便挑。

顺着这两个问题往下推：agent 放本地，状态存本地，代码也在本地，远程机器这一层就可以去掉。剩下要解决的是怎么跟公司内部系统（行云、JoySpace、Coding）对接，以及怎么把七个阶段串成一条可靠的流水线。

## 整体架构

Poria 是一个 Tauri v2 桌面应用。前端 React 19 + TypeScript + Ant Design 6，后端 Rust，打包成 macOS 原生窗口。

选 Tauri 而不是 Electron，原因是包体和内存。Tauri 用系统自带的 WebView，不捆绑 Chromium。macOS 上的 WKWebView 渲染性能够用，Poria 的前端没有复杂的画布或动画需求。

Rust 后端拆成六个 crate，加一个 Tauri 壳：

```
poria-core           # 类型、状态机、门禁、风险分级、合约、产出物定义
poria-infrastructure # 认证、配置、指标、SQLite
poria-resources      # Claude agent 池、终端、worktree、输出护栏
poria-channels       # Coding / JoySpace / 行云 / JME / 缺陷 HTTP 客户端
poria-skills         # 七个阶段的 skill 实现
poria-commands       # 流水线调度、错误分类、回滚
poria-desktop        # Tauri IPC 壳，#[tauri::command] 全在这里
```

依赖方向是单向的：`poria-core` 不依赖其他 poria crate，`poria-channels` 和 `poria-resources` 依赖 `poria-core`，`poria-skills` 依赖 `poria-resources` 和 `poria-channels`，`poria-commands` 只依赖 `poria-core` 和 `poria-infrastructure`，`poria-desktop` 把所有 crate 组装在一起。

这个分层的目的是隔离变更范围。改一个 HTTP 客户端（比如 Coding 的 MR 接口），只动 `poria-channels`，不影响状态机和调度逻辑。改 Claude CLI 的调用参数，只动 `poria-resources`，不影响 skill 逻辑。

前端通过 Tauri 的 `invoke` 调用 Rust 命令。IPC 命令全部定义在 `src-tauri/src/commands/` 下，按领域分文件：`auth.rs`、`demands.rs`、`pipeline.rs`、`repos.rs`、`skills.rs` 等。前端侧用 `src/lib/tauri.ts` 封装 `invoke`，TypeScript 的 camelCase 参数自动映射到 Rust 的 snake_case。

## 七阶段流水线

一条流水线从需求开始，到提交合并请求结束，固定七个阶段：

| 阶段 | 做什么 | 产出 |
|---|---|---|
| 初始化（Init） | 从 JoySpace 导出需求 PRD 和后端 TRD | `PRD.md`、`BACKEND_TRD.md` |
| 需求评审（ReviewPrd） | Claude 读 PRD，生成澄清问题清单 | `PRD_REVIEW.md` |
| 技术设计（Design） | Claude 基于 PRD 和后端 TRD 生成前端技术方案 | `TRD.md` |
| 工作区（Workspace） | 创建 git worktree + feature 分支 | `feature_<demand_code>` 分支 |
| 开发（Dev） | Claude 按 TRD 改代码 | 代码变更 + `TASK.md` |
| 代码审查（Cr） | Claude 自审代码变更 | `CR.md` + 评分 |
| 部署（Deploy） | commit → push → EasyCI 绑定 → 创建 MR | Coding 上的合并请求 |

所有文档产出物存在 `~/.poria/projects/<demand_code>/` 下。阶段顺序固定，定义在 `crates/poria-skills/src/stage_skill_map.rs` 的 `STAGE_ORDER` 数组里。

每个阶段对应一个 skill，skill 实现 `poria_core::contracts::Skill` trait：

```rust
#[async_trait]
pub trait Skill: Send + Sync {
    fn skill_id(&self) -> &str;
    async fn execute(&self, ctx: &SkillContext) -> Result<SkillOutput>;
}
```

重点看 `execute` 方法：接收一个 `SkillContext`（包含需求信息、仓库路径、worktree 路径、之前阶段的产出物），返回 `SkillOutput`（阶段状态、产出物路径、耗时、成本）。每个 skill 内部决定要不要调 Claude、调几次、用什么工具集。

## 两层状态机

流水线有两层状态：pipeline 级和 stage 级。两层都用静态转移表实现，定义在 `crates/poria-core/src/pipeline/state_machine.rs`。

Pipeline 级状态：

```
Created → Running / Cancelled
Running → WaitingMerge / Blocked / Failed / Cancelled
Blocked → Running / Cancelled
WaitingMerge → Completed / Failed / Cancelled
```

Stage 级状态：

```
Pending → Running / Skipped
Running → Completed / Failed / Blocked
Failed → Running     （重试）
Blocked → Running    （人工解除后继续）
```

转移函数的实现方式是查表：`PIPELINE_TRANSITIONS` 和 `STAGE_TRANSITIONS` 是 `const` 静态数组，`transition_pipeline()` / `transition_stage()` 遍历数组查当前状态到目标状态是否合法，不合法返回 `InvalidTransitionError`。

```rust
const STAGE_TRANSITIONS: &[(StageStatus, StageStatus)] = &[
    (StageStatus::Pending, StageStatus::Running),
    (StageStatus::Pending, StageStatus::Skipped),
    (StageStatus::Running, StageStatus::Completed),
    (StageStatus::Running, StageStatus::Failed),
    (StageStatus::Running, StageStatus::Blocked),
    (StageStatus::Failed, StageStatus::Running),
    (StageStatus::Blocked, StageStatus::Running),
];
```

这段代码展示了 stage 级全部合法转移。注意 `Failed → Running` 和 `Blocked → Running`：前者用于自动重试或手动重试，后者用于人工介入解除阻塞后继续执行。

用静态数组而不是 `match` 表达式或状态模式，好处是转移规则集中在一个地方，加一条新转移就是在数组里加一行，不需要改调度逻辑。

### 门禁

每个阶段出口有门禁检查，定义在 `crates/poria-core/src/pipeline/gates.rs`。9 个默认 gate 分三组触发：StageEntry（进入前检查）、StageExit（完成后检查）、Deploy（部署前检查）。

CR 评分是一个典型的门禁：代码审查阶段产出 `CR.md` 和评分，如果评分低于阈值，gate 把流水线回退到 Dev 阶段重新生成代码。

### 风险分级

`crates/poria-core/src/pipeline/risk_classifier.rs` 按变更内容做风险分级：

- 改了 CI 配置文件或 Dockerfile → Critical
- 改了配置文件且 diff 行数多 → High
- 其他按 diff 行数和文件数判断 → Medium / Low

风险等级影响门禁的严格程度和审批流程。

## Claude CLI 本地编排

Poria 不调 Claude API，而是用本地的 Claude CLI 作为子进程。实现在 `crates/poria-resources/src/claude/cli_sdk.rs`。

调用方式是 spawn `claude` 进程，传 `-p`（非交互模式）和 `--output-format stream-json`：

```rust
// cli_sdk.rs 核心调用逻辑（简化）
let mut cmd = Command::new("claude");
cmd.arg("-p")
   .arg("--output-format").arg("stream-json")
   .arg("--verbose")
   .arg("--system-prompt").arg(&system_prompt)
   .arg("--max-turns").arg(max_turns.to_string());

// 按阶段设置工具白名单
if !allowed_tools.is_empty() {
    cmd.arg("--allowedTools").arg(allowed_tools.join(","));
}

// 工作目录设为 worktree 路径
cmd.current_dir(&worktree_path);
```

重点看工具白名单的设计。每个阶段允许的工具集不同：

| 阶段 | 允许的工具 | 原因 |
|---|---|---|
| ReviewPrd | Read, Grep | 只需要读 PRD，不能改文件 |
| Design | Read, Grep, Write | 需要写 TRD.md |
| Dev | Read, Write, Edit, Bash, Glob, Grep | 需要改代码、跑命令 |
| Cr | Read, Grep | 只审查，不改代码 |

工具白名单限制了每个阶段的 agent 能做什么，防止需求评审阶段的 agent 去改代码，或者代码审查阶段的 agent 去改文件。

### Agent 池

`crates/poria-resources/src/claude/agent_pool.rs` 用 `tokio::sync::Semaphore` 控制 Claude CLI 的并发数。每次 dispatch 一个 agent 任务，先获取 semaphore permit，任务完成后释放。

每个阶段有独立的预算和超时配置：

| 阶段 | 预算上限 | 最大轮次 | 超时 |
|---|---|---|---|
| ReviewPrd | $1 | 10 | 5 分钟 |
| Design | $3 | 20 | 10 分钟 |
| Dev | $10 | 100 | 30 分钟 |
| Cr | $5 | 30 | 15 分钟 |

超过预算或轮次自动停止。返回值 `AgentTaskResult` 包含 success 标志、结果文本、session_id、实际花费和消息列表。

### 输出护栏

agent 写完代码后，`crates/poria-resources/src/output_guard.rs` 做三项检查：

1. **文件范围**：agent 改的文件必须在 TRD 声明的 scope 内（glob 匹配）。改了 scope 之外的文件，标记为越界。
2. **diff 大小**：变更超过 500 行触发警告。
3. **依赖安全**：检查新增的依赖是否在 blocked list 里。

## 本地存储

所有数据都在本地，不依赖远程服务器存储：

```
~/.poria/
├── auth.json                              # SSO Cookie
├── config.json                            # 应用配置
├── repos/<scope>/<name>/                  # 仓库克隆
├── projects/<demand_code>/                # 需求文档产出
│   ├── PRD.md
│   ├── PRD_REVIEW.md
│   ├── TRD.md
│   ├── BACKEND_TRD.md
│   ├── TASK.md
│   └── CR.md
└── worktrees/<pipeline_id>/<repo_name>/   # git worktree

~/Library/Application Support/com.poria.desktop/
└── poria.db                               # SQLite 数据库
```

仓库克隆到 `~/.poria/repos/` 下，按 scope 和 name 分目录。每条流水线创建独立的 git worktree 到 `~/.poria/worktrees/<pipeline_id>/` 下，worktree 里是 feature 分支。这样多条流水线可以并行，互不干扰。

SQLite 存流水线状态、阶段记录、agent 调用日志。正式运行用 `~/Library/Application Support/com.poria.desktop/poria.db`，开发调试可以用 `workspace/db/poria.db` 覆盖。

文档产出按需求编号分目录。Init 阶段从 JoySpace 导出 PRD，后续阶段在同一个目录里追加 `PRD_REVIEW.md`、`TRD.md` 等。一条流水线跑完，这个目录就是完整的需求-设计-审查档案。

## 跟内部系统的对接

本地优先不意味着离线运行。Poria 需要跟三个内部系统交互：

- **行云**：拉需求列表和需求详情。`poria-channels/src/xingyun/` 实现。
- **JoySpace**：导出文档内容。SSO Cookie + POST `/v1/pages/content`。
- **Coding**：克隆仓库、推送代码、创建 MR。`poria-channels/src/coding/` 实现。

认证统一用 SSO Cookie，存在 `~/.poria/auth.json`。Cookie 过期或缺失时，前端提示登录，不会静默失败。

Deploy 阶段的顺序有硬约束：先 commit，再 push，再调 EasyCI 绑定变更，最后创建 MR。EasyCI 只接受 `SELECT` 模式绑定已有分支，branch 必须先推到远端。这个顺序写错会导致 EasyCI 报 `522721`（branch not found）。

## 回滚

每个阶段在执行过程中记录回滚指令，实现在 `crates/poria-core/src/pipeline/rollback.rs`。指令类型包括：

- `DeleteBranch`：删除 feature 分支
- `CloseMr`：关闭已创建的 MR
- `RevertCommit`：回退 commit
- `RemoveWorktree`：清理 worktree 目录
- `RevertMr`：撤回已合入的 MR

阶段失败时，调度器按该阶段记录的回滚指令列表逆序执行。只回滚当前阶段的操作，不影响之前已完成阶段的产出。

## 边界和代价

这套方案有明确的适用边界：

**只支持 macOS。** Tauri 用的是 WKWebView，Windows 和 Linux 的 WebView 实现不同，Poria 目前只构建 macOS 的 aarch64 和 x86_64 两个架构。

**依赖本机 Claude CLI。** 开发者需要在本地安装并登录 Claude CLI，`claude` 命令必须在 PATH 里。CLI 的版本升级可能影响 agent 行为。

**SSO Cookie 会过期。** 跟行云、JoySpace、Coding 的交互全靠 Cookie，过期后需要重新登录。自动续期没有做。

**单机并发有上限。** agent 池的 semaphore 控制并发数，但 Claude CLI 本身也有 token rate limit。跑多条流水线时，实际并发取决于 CLI 的限流策略。

**不适合大团队共享状态。** 所有数据在本地，没有中心化的流水线状态面板。每个开发者只能看到自己机器上的流水线。如果需要团队级别的统计和调度，需要另外搭服务端。

<!-- TODO: 这里可以补充实际跑通一条流水线的时间数据，以及跟远程方案的对比 -->

## 小结

Poria 的核心判断：在 AI 编码场景下，把 agent、状态、代码全部放在开发者本地，比丢到远程机器上更快、更可控。代价是放弃了中心化管理和跨平台支持。对于团队当前的规模和场景（macOS 开发机、前端交付为主），这个取舍成立。
