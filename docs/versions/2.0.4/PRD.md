# Poria 2.0.4 PRD：需求工作台与交付动线产品化

| 字段 | 值 |
|------|-----|
| 版本 | 2.0.4 |
| 状态 | Draft |
| 产品 | Poria（macOS 桌面端，行云需求 → Coding MR） |
| 基线 | 2.0.3（流水线可用，动线与视图逻辑未产品化） |
| 参考 | 竞品「轻舟」需求工作台；deepseek-harness 的 Host 投影 / HITL 注意力模型（思想借鉴，不引入 Cordis） |

---

## 1. Summary

2.0.4 不改六阶段交付能力本身，而把「能跑通」做成「好用」。核心是：以**一条需求交付**为根对象，打开后进入**需求工作台**（对话轨迹、文档、工作区、人机确认同上下文切换）；列表只负责发现与注意力，侧栏资源页降为设置与阻塞引导。成功标准是：用户少切 Tab、少找入口，从阻塞到恢复有唯一主按钮。

---

## 2. Contacts

| Name | Role | Comment |
|------|------|---------|
| Aure / heyongqi10 | Product Owner / Eng | 2.0.4 范围与验收最终拍板 |
| clawd-cook | Review | 架构与 IPC 边界 review |
| TBD | Design | 工作台线框与主 CTA 视觉（可后补） |
| TBD | Alpha 用户 | 本地生活前端交付同学，验证动线 |

---

## 3. Background

### Context

Poria 2.0.x 已具备端到端能力：SSO、行云需求、JoySpace 文档、六阶段 Skill、门禁、HITL、推分支并创建 MR。后端状态机与 crate 分层清晰，**「能不能交付」已验证**。

前端仍是「领域对象的 Tab 浏览器」：

- 侧栏平行摆看板 / 渠道 / 技能 / 仓库 / 工作区 / 设置
- 真正交付几乎全挤在看板：列表 → 嵌套详情 → HITL 卡片
- 文档在抽屉或 HITL 弹窗；工作区是依赖「上次选中流水线」的空 Tab
- `humanRequest` 只写 store，**不自动切回看板、不选中流水线、无全局待办**

用户感受是：**能用，但不好用**——不知道下一步点哪里，换页就丢上下文。

### Why now

1. 能力面已齐，继续堆功能边际收益低；不好用会直接挡住日活与口碑。
2. 云端竞品「轻舟」用「需求列表 → 需求工作台（对话 / PRD / TRD / 目录 / Diff / 终端）」证明同一类 Job 可以做成低摩擦产品。
3. deepseek-harness 等 agent 平台强调：Host 真相 + Client 投影 + HITL 一等面；Poria 缺的是桌面端的投影与动线层，不是再造插件运行时。

### Why this became possible

桌面端已有 pipeline 事件、HITL、`workspacePath`、项目文档目录。2.0.4 主要是**重组信息架构与视图逻辑**，不必先重写 Rust 状态机。

---

## 4. Objective

### What's the objective

让日常交付者在 Poria 里完成「找需求 → 启动 → 等人确认 → 看产物 → 等到 MR」时，**始终知道自己在哪、下一步唯一该做什么**。

### Why it matters

- 对公司：缩短「学完 Poria 才能用」的成本，提高真实交付次数。
- 对用户：少在设置 / 仓库 / 看板之间猜；阻塞时立刻被带到可处理面。
- 对战略：Poria 定位是「AI 交付桌面端」，产品体验必须等于交付工作台，而不是运维控制台。

### Key Results（SMART）

| KR | 指标 | 目标（相对 2.0.3） | 测量方式 |
|----|------|-------------------|----------|
| KR1 | 主路径完成率 | Alpha 用户独立完成「未开始 → 启动 → 处理一次 HITL → 打开文档切面」≥ 90% | 观察式可用性测试（n≥5） |
| KR2 | 阻塞恢复时延 | 从 HITL 产生到用户看见可操作面板 ≤ 3 秒（同机已打开 App） | 桌面内计时 + 事件日志 |
| KR3 | 无关导航 | 单次交付中「为找进度而切侧栏资源 Tab」次数中位数 ≤ 1 | 会话备注 / 录屏 |
| KR4 | 主观易用 | SUS 或 5 分「我知道下一步」≥ 4.0 | 测后问卷 |

非目标（本版本不承诺）：云端协作、多用户实时、内嵌完整终端/IDE、花费分析大盘、对标轻舟全部切面。

---

## 5. Market Segment(s)

### For whom（按 Job，非人口统计）

| Segment | Job to be done | 约束 |
|---------|----------------|------|
| **主：本地前端交付者** | 把行云需求尽快变成可审的 Coding MR，中间少丢上下文 | 本机 Claude CLI；已登记前端仓；依赖 JoySpace PRD / 后端 TRD 链接 |
| **次：偶尔代跑的 TL / 支援** | 打开别人卡住的流水线，看清阶段与待确认项并处理 | 同一 SSO；不熟悉 Poria 设置细节 |
| **非目标：纯产品 / 纯后端无前端仓** | — | 2.0.4 仍以前端交付主路径为准 |

### Constraints

- 必须继续在 **Tauri 桌面窗** 内完成（Vite `:1420` 无 IPC，不算验收环境）。
- 不扩大 opener / shell 权限范围，除非另开安全评审。
- 不修改 `submodules/`；竞品与 harness 仅作参考。
- Node 24.20.0 / pnpm 11.23.0 / 既有六阶段 `STAGE_ORDER` 保持不变。

---

## 6. Value Proposition(s)

### Jobs / gains / pains

| 用户要完成的事 | 今天的痛 | 2.0.4 提供的得 |
|----------------|----------|----------------|
| 找到「该我处理」的交付 | HITL 可能发生在别的 Tab，无全局提示 | 待我处理队列 + 自动进入工作台对应面 |
| 一边看文档一边看进度 | 文档抽屉、详情、工作区分裂 | 同需求下 Tab 切面：轨迹 / 文档 / 工作区 |
| 知道当前阶段下一步 | 状态散落在卡片与 HITL | 顶栏阶段标签 + **唯一主 CTA** |
| 启动前补齐仓库 / 登录 | 向导与设置来回跳 | 就绪门禁：缺什么就引导补什么，补完回原任务 |

### Better than alternatives（Value Curve 要点）

| 能力 | 行云 / Coding 原生 | 轻舟（云） | Poria 2.0.3 | Poria 2.0.4 目标 |
|------|-------------------|------------|-------------|------------------|
| 本机 Claude + 本地 worktree | 低 | 中（云环境） | 高 | **保持高** |
| 需求级工作台（同上下文多切面） | 低 | **高** | 低 | **拉高到可用** |
| 固定六阶段门禁交付 | 低 | 中（工作流） | 高 | **保持高** |
| 阻塞时注意力与主 CTA | 低 | 高 | 低 | **拉高** |
| 离线 / 内网桌面可控 | 中 | 低 | 高 | **保持高** |

我们不和轻舟拼云 IDE；我们用桌面优势，把「需求工作台」产品化到同一清晰度。

---

## 7. Solution

### 7.1 UX / User flows

#### 信息架构（两层）

```text
壳（发现与配置）
├── 需求列表（原「看板」升级：发现 + 待我处理）
├── 设置（合并 Claude、登录、渠道说明等低频项）
└── 仓库（可保留独立页，但启动阻塞时以内嵌引导优先）

点开一条需求 / 流水线 → 需求工作台（新上下文，不换「产品」）
├── 顶栏：需求名 · 当前阶段 · 唯一主 CTA
├── 切面 Tab（同 pipelineId）
│   ├── 轨迹（对话式阶段流 / 事件 / 流式输出）
│   ├── 文档（PRD / PRD_REVIEW / TRD / BACKEND_TRD / TASK / CR）
│   ├── 工作区（路径、打开 Finder、只读文件树或入口）
│   └── 确认（HITL：澄清编辑 / resume / skip / cancel）— 有待办时高亮或自动切入
└── 返回列表（保留 selectedPipelineId，可再进）
```

渠道 / 技能：2.0.4 **移出主航道**（收进设置子页或「关于交付能力」只读），避免与交付动线抢注意力。

#### 主路径 A：从未开始到运行

```text
列表「未开始」→ 启动向导（backendTrdUrl 等）
  → 就绪检查失败？→ 内嵌「去登录 / 去登记仓库 / 去配 Claude」→ 回向导
  → 提交成功 → 自动进入该需求工作台 · 轨迹切面
  → 顶栏主 CTA 随阶段变化（运行中可显示「查看确认」若已阻塞）
```

#### 主路径 B：HITL 恢复

```text
任意壳页面收到 human:request
  → 全局角标 / 通知（待我处理 +1）
  → 若用户在本机前台：可选自动跳入该工作台 · 确认切面（可设置「仅角标不跳转」）
  → 处理 resume / skip / cancel 或编辑澄清
  → 回到轨迹切面看续跑
```

#### 主路径 C：看产物与工作区

```text
工作台内切「文档」→ 按产物列表打开 Markdown（只读为主；评审编辑仍走确认切面）
工作台内切「工作区」→ 展示 workspacePath / worktreePath → 一键打开目录
（不做轻舟级云终端 / 全量 Diff 编辑器于本版）
```

#### 线框要点（实现前可出低保真）

1. **列表**：状态筛选（全部 / 进行中 / 待我处理 / 已完成）+ 行：名称、阶段点、主操作「进入」。
2. **工作台顶栏**：左返回；中标题+阶段；右主 CTA（文案由 ViewModel 给出）。
3. **切面 Tab**：轨迹 | 文档 | 工作区 | 确认（确认仅在有 HITL / waiting_merge 时强调）。

### 7.2 Key Features

#### F1. Pipeline / Demand ViewModel（视图逻辑层）

- **输入**：pipeline、stages、`humanRequest`、auth、repos 就绪、config（claude_path）。
- **输出**：
  - `surface`：`list` | `workbench`
  - `mode`：`setup` | `running` | `awaiting_human` | `waiting_merge` | `failed` | `done`
  - `primaryCta`：文案 + action id（如 `open_hitl` / `resume` / `open_docs` / `open_mr`）
  - `badges`：待确认数量等
- React 页面只渲染 ViewModel，禁止各组件自行从原始事件拼「当前该干什么」。

#### F2. 需求工作台（Workbench）

- 进入条件：选中 `pipelineId`（或启动成功后强制选中）。
- 切面状态可进全局 UI state（如 `workbenchTab`），Persistent 或可恢复。
- 轨迹切面：迁移现有 `StageProgress` / `StreamOutput` / `EventStream` / `GateResults`，叙事上偏「会话时间线」，不要求真·多轮聊天协议。
- 文档切面：统一读取 `~/.poria/projects/<demand_code>/` 产物列表与预览（复用 / 收敛 `DemandProjectDrawer`）。
- 工作区切面：收敛现 `WorkspacePage`，寄生于当前 pipeline，不再作为与交付无关的空壳 Tab。
- 确认切面：收敛 `HumanLoopCard` / `PrdReviewEditor` 等。

#### F3. 列表与注意力

- 列表承担发现；「待我处理」聚合 `Blocked` + 有 `humanRequest` / HITL 泳道逻辑（复用 `hitlLane` 思路）。
- `human:request`：**至少**更新待办队列与角标；默认策略「跳入工作台确认切面」（设置可关）。
- 选中流水线时不再误清关键 HITL 上下文（修正「一点列表就丢 humanRequest」类行为）。

#### F4. 就绪门禁（Setup gate）

- 启动前检查：已登录、前端仓可用、`backendTrdUrl`、Claude 可解析。
- 失败时在向导内展示缺失项与跳转；返回后状态保留。
- 不在未登录时打 `list_demands`（保持现有安全约束）。

#### F5. 壳信息架构收敛

| 2.0.3 | 2.0.4 |
|-------|-------|
| home 看板兼详情 | 列表 ↔ 工作台分离 |
| channels / skills 主侧栏 | 移出主航道或降为设置只读 |
| workspace 独立 Tab | 并入工作台切面 |
| repos / settings | 保留；启动阻塞优先内嵌引导 |
| PipelineSidebar 死代码 | 删除或真正接入，禁止双轨 |

#### F6. 明确不做（2.0.4 Out of Scope）

- 云端会话续聊、多用户分享工作台
- 内嵌完整终端、云 Diff 编辑器、打开第三方 IDE 深度集成
- 项目级花费统计大盘（可在轨迹展示已有 cost 字段即可）
- 改 `STAGE_ORDER`、新增流水线阶段、改 EasyCI / Coding 契约
- 重写为 Cordis / 插件运行时

### 7.3 Technology（相关处才写）

- 前端：现有 React 19 + reducer store；新增 ViewModel 模块（纯函数 + 订阅），不引入第二套全局状态库。
- IPC：优先复用现有 `invoke` / 事件；若需「待办列表」聚合，可先前端派生，不够再加只读 command。
- 后端：默认无强制 crate 变更；若 HITL 事件载荷不足以驱动导航，再最小扩展事件字段。
- 设计 token：继续 `src/styles.css` 餐厅暖色与 `PageFrame`，工作台避免再洗成红/奶油仪表盘。

### 7.4 Assumptions

| # | 假设 | 若不成立 |
|---|------|----------|
| A1 | 用户心智是「一条需求一条交付」，可接受列表→工作台两层 | 改回强看板，但仍需全局 HITL 注意力 |
| A2 | 轨迹不必真·Chat 协议，时间线足够 | 再评估轻量「继续补充上下文」输入（可能进 2.0.5） |
| A3 | 文档只读预览 + HITL 内编辑够用 | 文档切面加重编辑能力 |
| A4 | Alpha 用户主要在单窗口前台使用 | 加强系统通知与角标，弱化自动跳转 |
| A5 | 渠道/技能移出主航道不伤害日常交付 | 保留设置入口即可 |

---

## 8. Release

### Effort（相对时间，非排期承诺）

| 阶段 | 内容 | 量级 |
|------|------|------|
| P0 | ViewModel + 列表/工作台分裂 + HITL 注意力 + 顶栏主 CTA | 约 1–2 周 |
| P1 | 文档 / 工作区切面收敛、就绪门禁、侧栏收敛、死代码清理 | 约 1 周 |
| P2 | 轨迹叙事打磨、设置项「HITL 自动跳转」、可用性测试修问题 | 约 0.5–1 周 |

整体目标窗口：**约 2–4 周**到可发 2.0.4（视设计与测试带宽）。

### Version slice

| 放入 2.0.4 | 留待后续 |
|------------|----------|
| 需求工作台四切面（轨迹/文档/工作区/确认） | 真·多轮续聊输入框 |
| ViewModel + 主 CTA | 内嵌终端 / Diff / Open IDE |
| HITL 角标与默认跳转 | 花费统计页 |
| 列表「待我处理」 | 知识库、智能运维类云能力 |
| 壳 IA 收敛 | 多仓工作台高级可视化 |

### Launch checklist（验收）

1. 未登录列表不打行云；登录后可筛「待我处理」。
2. 启动成功必进工作台轨迹切面；顶栏有阶段与主 CTA。
3. 制造一次 HITL：在设置页也能看到角标，并可一键进入确认切面完成 resume。
4. 文档切面能打开 `PRD.md` / `PRD_REVIEW.md` / `TRD.md`（有则显示）。
5. 工作区切面展示路径且能打开目录；无选中流水线时有空态引导回列表。
6. `pnpm typecheck` 通过；桌面窗（非 `:1420` 浏览器）手测主路径 A/B/C。
7. 版本号与发布说明写明：2.0.4 为体验与动线版本，交付阶段语义与 2.0.3 兼容。

### Risks

| 风险 | 缓解 |
|------|------|
| 大改 Shell 引入回归 | 工作台先挂在 home 路由下，PersistentTab 策略写清；分 PR：ViewModel → 壳 → 切面 |
| HITL 自动跳转打扰 | 设置开关；默认开给 Alpha，可关 |
| 与轻舟对标期望过高 | Release 说明写清 Out of Scope |

---

## Appendix A — 竞品与架构启发（非需求正文）

- **轻舟**：需求是根；工作台 Tab 是切面；顶栏阶段主 CTA；对话是脊柱。
- **deepseek-harness**：Host 真相 + Client 投影；HITL 一等面。Poria 用 ViewModel 落地投影，不引入 Cordis。
- **Poria 既有优势**：本机 worktree、固定门禁、SSO 深集成——2.0.4 予以保留并被工作台托住。

## Appendix B — 文档位置

- 本 PRD：`docs/versions/2.0.4/PRD.md`
- 架构真源：`ARCHITECTURE.md` / `AGENTS.md`（实现时若 IA 变更，需同步更新侧栏描述）
- 竞品截图：`competitor/轻舟/`
