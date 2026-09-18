# 项目工作区软链与标准 Claude skill — design

## Boundaries

- **In**: `run_init_stage` 工作区根与 worktree 路径；`WorktreeResource` 根目录；Init output 的 `workspacePath` / `worktreePath`；ReviewPrd / Design / Dev / Cr 的 Agent `cwd` 与 `-p` 内容；随包 `skills/<name>/SKILL.md`；工作区 `.claude/skills` 软链与 `CLAUDE.md`；侧栏「工作区」+ opener。
- **Out**: 后端进 `pipeline.repos`；文档进 git；旧 `~/.poria/worktrees` 迁移；用户 skill overlay；工作区文件树；一场 Claude 长会话；Deploy 改成 skill。

Init / Deploy 仍是 Rust `Skill`。四个 Claude 阶段保留 Rust 薄封装（拼短指令、设 cwd、工具白名单、验收产物存在），正文迁到 `SKILL.md`。

## Layout

```
~/.poria/projects/<demand_code>/          # 真源
  PRD.md  BACKEND_TRD.md  PRD_REVIEW.md  TRD.md  TASK.md  CR.md

~/.poria/workspaces/<pipeline_id>/        # Claude cwd
  CLAUDE.md                               # Init 写入，之后不改
  PRD.md -> projects/.../PRD.md           # 尚未生成的产物也先链上，写入即落到 projects
  BACKEND_TRD.md -> ...
  PRD_REVIEW.md -> ...
  TRD.md -> ...
  TASK.md -> ...
  CR.md -> ...
  .claude/skills/review-prd -> <bundled>/review-prd
  .claude/skills/gen-trd -> ...
  .claude/skills/gen-code -> ...
  .claude/skills/code-review -> ...
  <frontend_repo>/                        # feature worktree
  <backend_repo>/                         # detached worktree
```

`WorktreeResource` 的 `workspace_root` 改为 `~/.poria/workspaces`（不再是 `~/.poria/worktrees`）。现有 `path = root/pipeline_id/repo_name` 正好落在工作区根下。路径守卫同步改成「必须在 `~/.poria/workspaces` 下」。

工作区本身不 `git init`，避免和两个 worktree 抢根。Claude 以该目录为 `current_dir` 加载 `.claude/skills`。若 CLI 只认 git 根，再评估 `--add-dir` 或把 `.claude` 链进前端 worktree（禁止把 skill 提交进仓）；第一期按 cwd 发现实现。

## Bundled skills

权威目录：仓库 `skills/<name>/SKILL.md`（`name` = `stage_skill_id` 去掉 `skill:`）。

| Stage | id | 目录 |
|---|---|---|
| ReviewPrd | `skill:review-prd` | `skills/review-prd/` |
| Design | `skill:gen-trd` | `skills/gen-trd/` |
| Dev | `skill:gen-code` | `skills/gen-code/` |
| Cr | `skill:code-review` | `skills/code-review/` |

从现有 `crates/poria-skills/src/prompts/*.md` 迁入 `SKILL.md`（YAML `name`/`description` + 正文）。去掉等待用户、提问、HITL。写明：资料在工作区根软链；改代码只进前端 worktree；后端目录只读；产物写工作区根文件名（即软链目标）。

Tauri `bundle.resources` 打进包。运行时解析随包目录（dev 用仓库 `skills/`，release 用 resource dir），Init 对其做 `symlink`。不要拷贝。

## Init data flow

1. 现有：JoySpace → `~/.poria/projects/<demand_code>/`；同步两仓。
2. `mkdir ~/.poria/workspaces/<pipeline_id>/`。
3. 对六个产物文件名做指向 projects 的软链（目标文件可暂不存在）。
4. 软链四个 skill；写 `CLAUDE.md`。
5. `WorktreeResource::create` 前端；`create_detached` 后端。根为 `~/.poria/workspaces`。
6. Init output 增加 `workspacePath`。`worktreePath` 仍是前端 git worktree（Deploy 用）。`backend_context.local_path` 仍是后端 worktree。

任一步失败 → Init Failed。回滚删除整个 `~/.poria/workspaces/<pipeline_id>/`（含 worktree），不删 projects。

## Claude stages

`AgentTaskInput.worktree_path`（实际是 cwd）改为 `workspacePath`。

短 `-p` 模板（示例）：需求号、工作区根、前端/后端目录、点名 skill。不再 `--system-prompt` 塞旧 Prompt 全文。非交互约束写进各 `SKILL.md` 与工作区 `CLAUDE.md`。

工具白名单仍由各 skill 的 `extra_tools` 提供（ReviewPrd 已含 Write；Dev 含 Bash）。Rust 在 dispatch 后仍检查 projects 下产物存在；可保留「若误写到 worktree 根则 adopt」作为护栏，正常路径应走软链。

Deploy 仍只提交前端 worktree；`assert` 路径在 `workspaces/` 下。

## Sidebar

`ViewType` 增加 `workspace`。资源分组：渠道 / 技能 / 仓库 / **工作区**。页展示选中流水线的 `workspacePath`（来自 pipeline detail / Init output）。用已有 `tauri-plugin-opener` 打开目录。未选中流水线：文案引导先打开看板条目。不做文件树。

## Compatibility

不为旧 `~/.poria/worktrees/<id>` 流水线做迁移。新 Init 只写 workspaces。

## Trade-offs

- 工作区非 git 根：skill 发现依赖 cwd；比给工作区再套一层 git 更干净。
- 薄 Rust 封装保留：状态机、工具白名单、产物验收仍在 Poria；`SKILL.md` 只承载阶段做法。
- 侧栏只开 Finder：少做一套文件 UI，工作区仍可从桌面进入。
