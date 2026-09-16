# Internalize harness-o2o-fe into Poria: Dev Workflow Orchestration

## Goal

将 `@jd/harness-o2o-fe` 的核心研发流程能力内化到 Poria app，实现从需求澄清到代码审查的全流程闭环。通过 Poria pipeline 引擎编排流程、调用本机 Claude CLI + 内置 skill 处理 AI 任务、直接调用 channel/resource 处理平台操作，消除对全局安装 harness npm 包的依赖。

## Principles

1. **非必要不用 AI** — 纯机械操作（读文档、Git 操作、提 MR、部署）由 channel/resource 直接处理
2. **Claude 是工具而非编排者** — pipeline 引擎驱动流程，Claude 只处理具体 AI 任务
3. **Skill 内置于 app** — 不依赖全局包，skill prompt 作为 app 资源打包，对代码库无感
4. **每个 AI 阶段无状态** — 所有上下文通过 skill prompt + 输入文件提供给 Claude
5. **代码库无感** — 所有 skill 和流程数据存储在 app 内部，不在代码库中创建任何 harness 目录或配置文件

## User Flow

```
注册代码库 → 配置代码库信息
  ↓
拉取需求列表（从外部需求管理系统）
  ↓
所有需求默认「未开始」
  ↓
用户点击「开始」→ 弹出表单 → 选择目标代码库
  ↓
创建任务 → 开启 Pipeline:
  拉 worktree → 创建/关联分支 → 需求澄清(AI) → TRD 生成(AI)
  → 编码(AI) → 提交 → 部署 → 提 MR → CR(AI)
```

## Scope

### In Scope (v1 — 核心流程闭环)

覆盖研发主线的 4 个 AI 阶段 + 流程编排基础设施：

| Stage | 对应 harness skill | 类型 | 说明 |
|-------|-------------------|------|------|
| PRD Review | hfe-review-prd | AI | 需求前端落地澄清 |
| TRD Generation | hfe-gen-trd | AI | 技术设计文档生成 |
| Code Implementation | hfe-execute | AI | 文档驱动编码 |
| Code Review | hfe-cr-web | AI | Web 端代码审查 |

基础设施：
- Pipeline stage 定义与状态机扩展
- Claude CLI invoker（Rust，subprocess 管理）
- Skill 资源管理（内置 skill prompt 加载）
- Feature context（对应 harness 的 feature directory 概念）
- 前端 workflow UI（阶段展示、进度、人工审批卡点）

### Out of Scope (后续迭代)

- hfe-init（项目初始化/深度分析）
- hfe-gen-api（API 文档生成，作为 TRD 的子 skill 可后续接入）
- hfe-gen-ui（UI 资源获取，同上）
- hfe-cr-app（C 端 Taro 专项 CR）
- hfe-scan-app（全量扫描）
- Metrics 上报系统（`~/.harness/scripts/metrics/`）

## Requirements

### R1: Claude CLI Invoker

- Poria 后端（Rust）能够 spawn 本机 `claude` CLI 进程
- 支持传入: system prompt（skill 内容）、user prompt（阶段输入）、工作目录
- 调用方式: `claude -p "<prompt>" --system-prompt "<skill>" --allowedTools "<tools>" --output-format stream-json`
- 支持流式输出捕获（stream JSON 解析，实时反馈到前端）
- 支持超时控制、进程取消
- 支持并发会话（多个 Claude 进程同时运行，如多 SubAgent 并行编码）

### R2: Skill Resource System

- Skill prompt 文件作为 app 内置资源，打包在 binary 中或随 app 分发
- 每个 AI 阶段对应一个 skill prompt 文件
- Skill prompt 从 harness 现有能力迁移并精简，去除全局安装依赖相关的脚本调用
- 支持 skill 模板变量替换（如 `{{feature_dir}}`、`{{project_root}}`）
- 4 个 v1 skills: `prd-review`, `trd-gen`, `code-impl`, `cr-web`

### R3: Feature Context Management

- 每个研发需求对应一个 feature context，管理该需求各阶段的产出文件
- Feature context 数据存储在 app 数据目录中（SQLite + 文件系统），**不在代码库中创建任何文件**
- AI 阶段的产出文件（PRD_REVIEW.md, TRD.md 等）存在 app data 路径下
- 需要给 Claude 使用的文件通过临时目录传递（Claude 在 worktree 中工作时，skill 产出放在 worktree 内临时路径）
- 支持 feature context 的创建、查询、归档
- Feature context 关联: 需求 ID + 代码库 + Pipeline

### R4: Dev Workflow Pipeline

- 在 poria-core pipeline 引擎中定义完整研发流程
- 阶段顺序:

```
Workspace(创建 worktree/分支) → PRD Review(AI 需求澄清) → TRD Generation(AI 技术设计)
→ Code Implementation(AI 编码) → Commit(提交) → Deploy(部署) → MR(提 MR) → Code Review(AI CR)
```

- 非 AI 阶段直接由 channel/resource 执行:
  - Workspace: worktree resource + coding channel (创建分支、关联需求)
  - Commit: git 操作 (resource)
  - Deploy: xingyun/jme channel
  - MR: coding channel
- AI 阶段调用 Claude CLI + skill
- 每个阶段之间有 gate（前置条件检查）:
  - TRD gate: PRD_REVIEW.md 存在且 P0 问题已回答
  - Execute gate: TRD.md 存在
  - CR gate: 代码变更存在（MR 已提）
- 支持人工审批卡点（human-in-the-loop）:
  - PRD Review 后用户确认 scope
  - TRD 生成后用户审阅
  - 编码完成后用户审查
- 支持阶段跳过（用户手动标记某阶段不需要）
- 支持阶段重试（某阶段失败后重新执行）

### R5: Stage Implementations

#### R5.1: PRD Review Stage

- 输入: PRD 文档（从 JoySpace 获取或用户提供）
- 处理: 调用 Claude + `prd-review` skill
- 输出: PRD_REVIEW.md（模块分解 + P0/P1/P2 澄清清单）
- 人工卡点: 用户确认 scope、回答 P0 问题

#### R5.2: TRD Generation Stage

- 输入: PRD.md + PRD_REVIEW.md + 可选 API/UI 资源
- 处理: 调用 Claude + `trd-gen` skill
- 输出: TRD.md（技术设计文档）
- Gate: PRD_REVIEW.md P0 全部已回答
- 人工卡点: 用户审阅 TRD

#### R5.3: Code Implementation Stage

- 输入: TRD.md + PRD.md + 可选 API.md
- 处理: 调用 Claude + `code-impl` skill（编排 SubAgent）
- 输出: TASK.md（执行计划）+ 业务代码
- Gate: TRD.md 存在
- 人工卡点: 用户审阅 TASK.md 后才开始编码

#### R5.4: Code Review Stage

- 输入: Git diff（代码变更）
- 处理: 调用 Claude + `cr-web` skill
- 输出: CR report
- Gate: 代码变更存在

### R6: Frontend Workflow UI

- Workflow 面板展示当前研发流程状态（各阶段进度）
- 每个阶段: pending / running / waiting-review / completed / skipped / failed
- Claude 输出实时流式展示
- 人工审批卡点 UI（approve / reject / request-changes）
- Feature context 文件浏览（查看各阶段产出文件）

## Acceptance Criteria

- [ ] 能在 Poria 内发起一个完整的研发流程（PRD Review → TRD → Execute → CR）
- [ ] 每个 AI 阶段正确调用本机 Claude CLI 并传入对应 skill
- [ ] Claude 输出实时流式展示在前端
- [ ] Gate 机制正确阻止不满足前置条件的阶段执行
- [ ] 人工审批卡点能暂停流程并等待用户操作
- [ ] Feature context 正确管理各阶段产出文件
- [ ] 不依赖全局安装的 `@jd/harness-o2o-fe`
- [ ] 阶段可跳过、可重试
- [ ] 多 Claude 进程并发支持（编码阶段的 SubAgent）

## Constraints

- Claude CLI 必须已安装在本机（Poria 不负责安装 Claude）
- 使用 Claude CLI 的 `--output-format stream-json` 获取流式输出
- Skill prompt 总长度需控制在合理范围内以避免 context window 浪费
- Feature context 文件存储在项目目录内（不在 app 数据目录中）

## Task Structure

建议拆分为 parent + 4 个 child task：

| Child | 标题 | 可独立验证 |
|-------|------|-----------|
| child-1 | Claude CLI Invoker + Skill Resource System | 能 spawn Claude 进程、加载 skill、获取流式输出 |
| child-2 | Feature Context + Dev Workflow Pipeline | Pipeline 状态机、gate、feature context 管理 |
| child-3 | Stage Implementations (4 stages) | 各阶段能正确调用 Claude 并生产预期产出 |
| child-4 | Frontend Workflow UI | 阶段展示、流式输出、审批卡点 |
