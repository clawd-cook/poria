# Delivery workspace: kanban, Xingyun demands, repos, settings

## Goal

把 Poria 桌面端做成面向**前端研发**的交付工作台：先登记并克隆仓库，再用 SSO 拉取「指派给我」的行云需求，从需求右侧「开始」配置前端仓、后端仓+分支、JoySpace PRD，然后跑完整流水线。

主交付物始终是前端代码（功能分支、CR、MR）。后端仓不是第二条交付流水线，只按所选分支提供只读代码上下文，辅助生成前端实现。

## Background

当前桌面端（`src/components/Shell.tsx`）是 Pipeline / 技能 / 渠道 三栏，设置为 Pipeline 侧栏 Modal。提交入口是粘贴行云卡片链接（`SubmitBar` → `submit_pipeline`），只解析 URL 并创建空 Pipeline（`repos: []`）。

- SSO 由独立任务 `09-17-sso-login` 提供：凭据在 `~/.poria/auth.json`。本任务只消费登录态。
- Xingyun channel 已有 `getDemand` / 附件 / `resolvePrdLink`，没有需求列表。列表行为对齐 h2o-plugin `demands.list`（默认指派给我、keyword、分页）。
- `RepoConfig` 挂在 Pipeline 上，不是本机已登记仓清单。Worktree 假定 git root 已在本地。`pipeline.repos.length > 1` 会走 multi-repo 开发路径，因此后端上下文不能放进 `pipeline.repos`。
- git URL 路径解析已有 `repo_search_path_from_git_url`：`git@coding.jd.com:ls/ls-entrance.git` → `ls/ls-entrance`。

## Requirements

### R1: 四 Tab 信息架构

- 顶栏四个 Tab，顺序固定：**首页（任务看板）**、**需求列表**、**仓库列表**、**设置**
- 取代 Pipeline / 技能 / 渠道。技能页、渠道页本轮不作为主路径
- 设置从 Modal 升为独立 Tab
- 登录态在顶栏全局可见；设置 Tab 也可登录/登出。未登录时需求列表不拉数并引导登录

### R2: 首页任务看板

- 展示本机流水线，按状态分列：运行中 / 阻塞 / 待合并 / 完成 / 失败（其余归入「其他」或运行中前的创建态）
- 卡片可见需求名、需求编号、当前阶段、状态、更新时间
- 点击卡片在首页打开既有 Pipeline 详情（阶段进度、事件流、人工回路），提供返回看板；不另做第二套详情

### R3: 需求列表

- 已登录时用当前 SSO cookie + ERP 拉取行云需求
- 范围仅「指派给我」+ 关键字搜索 + 分页。不做「我提出的」或项目全量
- 每行：需求名称、编号、状态、接收人；右侧「开始」
- 未登录、cookie 失效、接口失败时给出去登录 / 重试，不展示假数据

### R4: 从需求开始流水线

- 「开始」进入配置：前端代码库 → 后端代码库+分支 → JoySpace PRD → 确认后创建并执行 7-stage 流水线
- 前端仓必选，来自已克隆成功的登记仓；不另选前端分支，使用该仓本地当前检出。前端仓是唯一交付对象
- 后端仓+分支必选、不可跳过。只读上下文，不开后端功能分支、不创建后端 MR、不作为 deploy 对象
- 前端与后端必须是两份不同的已登记仓
- PRD 为必填 JoySpace 链接。若卡片附件能经 `resolvePrdLink` 解析则预填，用户可改；解析不到则手动粘贴
- 提交后切到首页看板并选中新任务。不再以粘贴行云链接作为主入口

### R5: 仓库列表

- 只填 git URL（SSH/HTTPS）并克隆；不支持选择已有本地目录
- 根目录 `~/.poria/repos/`，按 **scope / 代码库** 分层，路径取自 git URL：`git@coding.jd.com:ls/ls-entrance.git` → `~/.poria/repos/ls/ls-entrance`
- 列表按 scope 分组：仓库名、git URL、本地路径、克隆状态（进行中 / 成功 / 失败）
- 失败可重试；同一规范化 URL 或同一 `scope/repo` 已存在则拒绝
- 登记时不标注前端/后端角色；角色只在「开始」向导里选
- 流水线对前端仓从该 git root 创建 worktree；后端仓在所选分支上只读检出/读取

### R6: 设置 Tab

- 承接现有设置：门禁阈值、超时、重试、数据目录
- 承接登录/登出与当前 ERP
- 本轮不扩展无关配置面

## Constraints

- 无登录不调行云接口；不重做 SSO，复用 `start_login` / `logout` / `auth:status-changed`
- 流水线阶段不变：`init → review_prd → design → workspace → dev → cr → deploy`
- 向导候选仓只来自本机已登记且克隆成功的仓库
- `pipeline.repos` 只含前端交付仓；后端上下文单独存储，避免误入 multi-repo 开发/CR/deploy
- 不修改 `submodules/`；需求列表对齐 h2o-plugin `demands.list` 的「指派给我」行为

## Out of Scope

- 技能页、渠道页作为主导航
- 粘贴行云链接提交条作为主入口
- 行云受理/沟通（communicate/accept）
- 后端仓的功能分支、CR、MR、部署
- 「我提出的」或项目全量需求列表
- 多仓拓扑（>1 个交付仓）与纯远程未克隆仓启动
- 向导内粘贴 PRD Markdown 或上传本地文件
- 登记已有本地工作副本
- 系统托盘、多窗口、CI 发布流程

## Acceptance Criteria

- [ ] 顶栏仅四个 Tab：首页、需求列表、仓库列表、设置；切换后各页状态不丢
- [ ] 仓库能登记 SSH 地址并克隆到 `~/.poria/repos/<scope>/<repo>`（示例 `ls/ls-entrance`），列表按 scope 分组显示路径
- [ ] 已登录可加载「指派给我」的需求（可搜索、分页）；未登录引导登录且不造数据
- [ ] 「开始」必须选不同的前端仓、后端仓+分支，并填写 JoySpace PRD 才能提交
- [ ] 有附件 PRD 时向导预填且可改；空 PRD 不能启动
- [ ] 提交后首页看板出现卡片；主工作区/功能分支/MR 只针对前端仓，不产生后端 MR
- [ ] 点击看板卡片打开既有 Pipeline 详情并可返回看板
- [ ] 设置 Tab 可改原配置项并可登录/登出
- [ ] `pnpm typecheck` 通过；涉及 Rust 时 `cargo check --workspace` 通过
