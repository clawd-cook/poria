# Poria 平台化架构设计 — PRD v4.3

> v4.2: L-audit 同步（PRD_INVALID 异常场景 + trdScope 定义）。

## Goal

将 Poria 从单机 CLI 原型（poria-mvp）升级为 **AINative 交付平台**：用户粘贴**行云卡片链接**作为唯一入口，系统静默执行完整开发流程（PRD 解析 → 技术设计 → 编码 → CR → 构建 → MR），门禁全部通过后**自动标记 MR 可合并 + 京ME 通知人工一键确认**（MVP 不做自动合并）。全程可观测、可重现、可断点续跑。仅在无法自主解决时通过京ME与对应员工交互。

---

## 核心需求

### R1: 行云卡片链接入口

- **唯一入口**：用户粘贴行云卡片链接（URL 中含 demandId + demandCode，projectId 从 API 响应获取）
- 系统通过 `XingyunChannel.getDemand()` + `resolvePrdLink()` 解析完整需求元数据
- **鉴权**：poria-auth SSO cookie 校验，未登录 → 引导 `poria auth login`
- **失效兜底**：链接无效/需求已关闭/无权限 → 明确错误信息 + 终止，不创建 Pipeline
- **元数据解析**：demandId, demandCode, name, status, projectId, PRD 链接, 卡片附件
- 解析成功后自动创建 Pipeline 实例，进入静默执行

### R2: 静默端到端 Pipeline

Pipeline 阶段复用 poria-mvp 已验证的 7-step 模型，改为**自动执行**：

| 阶段       | 输入             | 输出                           | 自动化程度                      |
| ---------- | ---------------- | ------------------------------ | ------------------------------- |
| init       | 行云卡片链接     | 项目目录 + PRD.md              | 全自动                          |
| review-prd | PRD.md           | PRD_REVIEW.md (P0/P1/P2)       | 全自动，P0 未答 → 京ME 通知产品 |
| design     | PRD + PRD_REVIEW | TRD.md                         | 全自动                          |
| workspace  | 项目配置         | Git worktree + 分支            | 全自动                          |
| dev        | TRD.md           | 代码变更                       | AI Agent 静默执行               |
| cr         | 代码 diff        | CR_REPORT.md + 门禁评分        | AI Agent 执行 + 门禁检查        |
| deploy     | 门禁通过         | Build + Push + MR + 标记可合并 | 全自动，**人工一键确认合并**    |

### R3: MR 合并准入门禁

**deploy 阶段不直接合并**，而是通过门禁检查后自动标记 MR 可合并，通知人工一键确认：

| 门禁项     | 条件                                          | 不通过处理            |
| ---------- | --------------------------------------------- | --------------------- |
| CI 构建    | build 成功 + 无编译错误                       | 重试 → 通知开发者     |
| 单测覆盖   | 新增代码测试覆盖率 ≥ 阈值（可配置，默认 80%） | 通知开发者            |
| CR 评分    | AI CR 评分 ≥ 阈值（可配置，默认 B+）          | 重跑 CR → 通知开发者  |
| 安全扫描   | 无高危/严重漏洞                               | 通知开发者 + 安全团队 |
| 变更量检查 | diff 行数 ≤ 阈值（可配置，默认 500 行）       | 标记需人工 review     |
| 合并冲突   | 无冲突                                        | 通知开发者            |

门禁全部通过 → 自动创建 MR → 标记为"门禁通过，可合并" → 京ME 通知 MR reviewer 一键确认合并。

### R4: 状态持久化与断点续跑

- **存储介质**：SQLite 本地文件（零部署、事务性好、单机够用）
- **持久化内容**：
  - Pipeline 元数据（id, demandRef, status, 创建/更新时间）
  - 每个 Stage 的状态（status, retryCount, input snapshot, output）
  - 事件日志（全部领域事件追加写入，用于回放）
- **断点续跑**：
  - 系统重启后从 SQLite 恢复所有 RUNNING/BLOCKED Pipeline
  - 找到最后一个 COMPLETED Stage，从下一个 Stage 恢复执行
  - Agent 执行中断 → 该 Stage 标记为 FAILED → 按重试策略恢复
- **事件回放**：从事件日志可完全重建任意 Pipeline 的历史状态

### R5: 可观测可重现

- 每个 Pipeline 阶段产生结构化事件日志
- Dashboard 实时展示所有 Pipeline 实例状态（P3 阶段实现）
- 任意阶段可回放输入/输出，支持重跑失败阶段（P3 阶段实现）
- **监控指标**（P3 阶段实现）：
  - Pipeline 成功率 / 失败率（按阶段统计）
  - Agent 执行耗时（按 Stage 统计）
  - LLM token 消耗量
  - 人工介入频率

### R6: 智能异常处理 + 京ME 人工回路

- AI Agent 执行中遇到无法解决的问题，自动分类
- 根据问题类型路由到对应员工的京ME
- 员工在京ME回复后，系统自动恢复 Pipeline 执行
- 超时未回复 → 升级通知 → 最终降级为手动处理
- **异常场景清单**：
  1. 行云链接无效/无权限 → 终止 + 通知用户
  2. PRD 无内容/格式异常 → 通知产品
  3. 编译错误 → 重试 ×3 → 通知开发者
  4. 测试失败 → 重试 ×3 → 通知开发者
  5. Agent 产出低质量代码（CR 评分不达标）→ 回退到 dev 阶段重新生成代码（最多 1 次 dev→cr 回退循环）→ 仍不达标则通知开发者
  6. Agent 超时（总执行时间 >30min 或连续 5min 无输出）→ kill + 重试 → 通知
  7. Git 权限/分支保护 → 通知运维
  8. 合并冲突 → 通知开发者
  9. 需求歧义（PRD P0 未答）→ 通知产品
  10. LLM API 限流/故障 → 等待重试 → 通知

### R7: 安全与审计

- **最小权限**：Pipeline 使用提交者的 SSO 身份操作，不使用超级账号
- **操作留痕**：所有 git 操作（commit, push, MR create）记录到事件日志，含操作人、时间、变更摘要
- **恶意链接防护**：行云链接 URL 校验（白名单域名 + 格式校验），拒绝非法输入
- **Agent 输出拦截**：
  - 禁止 Agent 修改非目标文件（超出 TRD 定义范围的文件变更 → 拒绝 + 告警）
  - 禁止引入已知恶意依赖（安全扫描门禁）
  - diff 超过阈值 → 强制人工 review
- **回滚机制**：
  - 每个 Stage 的 git 操作生成 rollback 指令
  - Pipeline 取消/失败（MR 未合并）→ 一键回滚（删除分支/关闭 MR）
  - Pipeline 取消/失败（MR 已合并）→ 自动创建 revert MR + 京ME 通知 reviewer 确认

### R8: 多仓库编排

- 单个需求可关联多个仓库，各仓库独立 worktree
- 仓库间有依赖时支持顺序编排
- 全部仓库完成后统一进入 deploy 阶段
- **依赖故障熔断**：仓库 A 失败 → 依赖 A 的仓库 B 自动跳过 → 通知开发者

---

## 约束

### C1: 技术栈

- Node.js (v24.20.0) + TypeScript，复用 poria SDK 包结构
- pnpm 11.23.0 monorepo
- SQLite（better-sqlite3）作为持久化

### C2: 复用优先（吸收迁移）

已有包 `@dj-lib/poria-*` 存在于 `submodules/poria/packages/` 中（已发布到 registry.m.jd.com），设计意图是将其**吸收迁移**到主仓 `packages/` 中并重命名为能力导向的包结构。具体映射：

| 来源包（submodules/poria/）       | 迁移目标（packages/）                      | 复用内容                                                |
| --------------------------------- | ------------------------------------------ | ------------------------------------------------------- |
| `@dj-lib/poria-plugin-sdk`        | `core/contracts/`                          | Channel/Resource 接口定义，直接复制并重构为四种能力契约 |
| `@dj-lib/poria-core` (channel)    | `core/pipeline/` + `infrastructure/store/` | 事件模型和 append/read/watch 原语，适配为 Pipeline 事件 |
| `@dj-lib/poria-core` (task)       | `core/types/`                              | TrellisTaskRecord schema 的字段参考                     |
| `@dj-lib/poria-core` (mem)        | `infrastructure/` (后续)                   | 跨会话记忆，P1 不迁移                                   |
| `@dj-lib/poria-channel-coding`    | `channels/coding/`                         | EasyCI HTTP 客户端代码，直接迁移                        |
| `@dj-lib/poria-channel-joyspace`  | `channels/joyspace/`                       | JoySpace export 逻辑，直接迁移                          |
| `@dj-lib/poria-channel-xingyun`   | `channels/xingyun/`                        | JACP 客户端代码，直接迁移，增加链接解析                 |
| `@dj-lib/poria-resource-terminal` | `resources/terminal/`                      | Shell exec 实现，直接迁移                               |
| `@dj-lib/poria-auth`              | `infrastructure/auth/`                     | SSO cookie 提取，直接迁移                               |

- 迁移方式：源码复制 + 重构（非 npm install），迁移后主仓不再依赖 `@dj-lib/` 外部包
- submodules/poria/ 作为只读参考，不作为运行时依赖

### C3: 并发模型

- **MVP: 单 Pipeline 串行**，队列排队
- 后续再开放有限并发（3-5 条）

### C4: 渐进式

- P1: CLI Pipeline（本地串行）
- P2: 京ME 人工回路 + 门禁
- P3: Dashboard + API + 监控
- P4: Apps (Next.js + Tauri)

---

## 非目标（本次不做）

- 多租户隔离
- 自定义 Pipeline 编排（固定 7-step）
- 多 Pipeline 并发（MVP 串行）
- 计费和配额管理
- 自动合并（MVP 止步于标记可合并 + 一键确认）

---

## Acceptance Criteria

- [ ] 入口为行云卡片链接，含完整的链接解析、SSO 鉴权、失效兜底协议
- [ ] Pipeline 状态机定义完整，覆盖正常流转 + 异常 + 重试 + 人工介入 + 恢复 + 取消
- [ ] MR 合并准入门禁规则明确（CI/测试/CR/安全扫描/变更量），门禁通过后自动标记 + 京ME 一键确认
- [ ] 状态持久化方案（SQLite），支持断点续跑（重启后恢复 RUNNING Pipeline）
- [ ] 安全与审计：最小权限、操作留痕、恶意链接防护、Agent 输出拦截、回滚机制
- [ ] 异常场景清单（10+ 场景）每个有明确处理策略
- [ ] 京ME 人工回路：触发条件、消息格式、恢复协议、超时升级
- [ ] 监控指标定义（成功率、耗时、token、人工介入率）
- [ ] 能力包拆分合理，每包独立可测
- [ ] 设计文档经用户评审通过
