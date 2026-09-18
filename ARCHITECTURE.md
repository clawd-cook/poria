# Architecture

Poria 是 Tauri v2 桌面壳 + React 前端 + Rust Cargo workspace。产品用法见 [README.md](README.md)；编码代理契约见 [AGENTS.md](AGENTS.md)。

## 仓库布局

```
poria/
├── src/                      # React 前端（Ant Design 6 + Ant Design X）
│   ├── components/           # 页面与流水线 UI
│   ├── hooks/                # usePipeline、useTauriEvents
│   ├── lib/                  # 类型与 invoke 封装（tauri.ts）
│   ├── state/                # reducer store
│   ├── theme.ts              # poriaTheme
│   └── styles.css            # 重置、隐藏滚动条
├── src-tauri/                # Tauri v2 壳与 IPC（crate: poria-desktop）
│   └── src/commands/         # 唯一的 #[tauri::command] 层
├── crates/
│   ├── poria-core            # 领域类型、状态机、门禁、风险分级
│   ├── poria-commands        # 流水线执行、错误分类、回滚
│   ├── poria-skills          # 六阶段内置 Skill
│   ├── poria-resources       # 终端、worktree、Claude Agent 池
│   ├── poria-channels        # Coding、JoySpace、行云、JME、缺陷
│   └── poria-infrastructure  # SQLite、鉴权、配置、日志、指标
├── skills/                   # 随包 Claude SKILL.md（review-prd / gen-trd / gen-code / code-review）
└── public/                   # 静态资源
```

`submodules/` 只作参考，不要改里面的文件。实现写在本仓库的 crate / `src/`。

## 依赖方向

不要反转：

```
poria-core
  ├── poria-infrastructure
  ├── poria-resources
  └── poria-channels → poria-skills
poria-commands          # 只依赖 core + infrastructure
src-tauri (poria-desktop)
```

| Crate | 职责 |
|---|---|
| `poria-core` | 领域类型、状态机、门禁、风险分级、多仓拓扑 |
| `poria-commands` | 流水线执行器、错误分类、回滚（**不是** IPC 层） |
| `poria-skills` | Init / ReviewPrd / GenTrd / GenCode / CodeReview / Deploy |
| `poria-resources` | 终端、worktree、Claude Agent 池、输出护栏 |
| `poria-channels` | Coding、JoySpace、行云、JME、缺陷 |
| `poria-infrastructure` | SQLite、鉴权、配置、日志、指标 |
| `poria-desktop` | Tauri 壳、`#[tauri::command]`、事件 emit |

共享 Rust 依赖写在根 `Cargo.toml` `[workspace.dependencies]`，crate 里用 `{ workspace = true }`。

## 前端与 IPC

侧栏视图：看板 `home`、渠道 `channels`、技能 `skills`、仓库 `repos`、工作区 `workspace`、设置 `settings`。没有单独的需求页；未开始列就是行云任务。

- IPC 封装只放在 `src/lib/tauri.ts`。前端 camelCase（`pipelineId`、`backendTrdUrl`）对应 Rust snake_case。
- `#[tauri::command]` 只写在 `src-tauri/src/commands/`，并在 `src-tauri/src/lib.rs` 注册。
- Cursor / 浏览器打开 Vite `:1420` **没有** `invoke` / `listen`。验证必须在 `poria-desktop` 窗口里做。
- 布局用 Ant Design token 和 `src/theme.ts`，不用 Tailwind。

侧栏「技能」只列出随包 `skills/*/SKILL.md`。Init 与 Deploy 是 Rust 阶段实现，不在这份清单里。

## 流水线状态

```
Created → Running → WaitingMerge → Completed
              ↓          ↓
           Blocked     Failed
              ↓          ↓
           Running    Cancelled
```

`Created` / `Running` / `Blocked` / `WaitingMerge` / `Failed` 均可取消。`Completed` 与 `Cancelled` 为终态。

阶段自身走 `Pending → Running → Completed`，也可进入 `Failed` / `Blocked` / `Skipped`。失败可按配置重试；阻塞则等人处理（`resume` / `skip` / `cancel`）。CR 评分不达标会回退到开发阶段，每个流水线只允许一次；第二次回退失败会升级为拦截。

## 六个阶段

没有独立的 Workspace 阶段。Init 会创建工作区与前后端 worktree，然后才进入评审。

| 阶段 | Skill | 产出 |
|---|---|---|
| 初始化 | `skill:init` | 导出 `PRD.md` / `BACKEND_TRD.md` 到 `~/.poria/projects/<需求编号>/`；创建 `~/.poria/workspaces/<pipeline_id>/`（文档与 Skill 软链 + `CLAUDE.md`）；拉起前端 feature worktree 与后端 detached worktree |
| 需求评审 | `skill:review-prd` | `PRD_REVIEW.md`（P0 / P1 / P2 澄清项） |
| 技术设计 | `skill:gen-trd` | 前端 `TRD.md` 与允许修改范围 |
| 开发 | `skill:gen-code` | 按 TRD 改前端 worktree，写出 `TASK.md` |
| 代码审查 | `skill:code-review` | `CR.md`，过评分 / 安全扫描门禁 |
| 部署 | `skill:deploy` | 提交 → 推送 → 绑定行云 EasyCI 变更 → 创建或复用 Coding MR |

文档真源在需求项目目录，不进 git worktree。前端 `TRD.md` 与后端 `BACKEND_TRD.md` 是两份文件，不要混用。

> [!NOTE]
> 评审、设计、开发、CR 通过 `claude -p` 非交互执行，cwd 是工作区根。部署顺序是 **先 push 再 SELECT 绑定**：EasyCI `createChange` 只能选择远端已存在的分支。

`submit_pipeline` 必须带 `backendTrdUrl`。`pipeline.repos` 只放前端仓；后端仓留在 `backend_context`。前端 `base_branch` 用登记仓库的 `default_branch`（默认 `master`）。

## 数据目录

| 内容 | 路径 |
|---|---|
| SSO Cookie | `~/.poria/auth.json` |
| 应用配置 | `~/.poria/config.json` |
| 托管克隆 | `~/.poria/repos/<scope>/<name>` |
| 需求 Markdown | `~/.poria/projects/<需求编号>/` |
| 流水线工作区 | `~/.poria/workspaces/<pipeline_id>/` |
| SQLite（正式） | `~/Library/Application Support/com.poria.desktop/poria.db` |
| SQLite（开发覆盖） | 仓库内 `workspace/db/poria.db` 存在时优先 |

`workspacePath` 是工作区根，不等于前端 `worktreePath`。后端 worktree 必须 `--detach`，以免锁住托管克隆上的同一分支。Init 失败时先 `git worktree remove`，再删工作区目录。

不要把 `~/.poria/`、`workspace/db/` 或 Cookie 提交进 git。

## 技术栈

| 层 | 技术 |
|---|---|
| 后端 | Rust 2021、Tokio、SQLite（rusqlite）、Serde |
| 前端 | React 19、TypeScript、Vite 8、Ant Design 6、Ant Design X |
| 桌面 | Tauri v2（`com.poria.desktop`） |
| Agent | Claude CLI 子进程池 |

本地可用 `PORIA_PIPELINE_FIXTURE=1` 跳过真实 Claude / 渠道调用。不要在生产路径打开这个开关。
