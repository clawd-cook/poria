# CLI Commands — PRD

> Child of: `09-16-poria-platform-arch`
> 依赖: pipeline-orchestrator（传递依赖 core-foundation、infra-persistence、channel-migration、resource-layer）

## Goal

实现 Poria CLI 命令层，提供用户与平台交互的全部入口。每个命令实现 `ICommand` 契约接口，通过 `IPluginLoader` 注册发现。

---

## 核心需求

### R1: pipeline 命令组

#### R1.1: `poria pipeline submit <xingyun-link>`

**主入口** — 用户粘贴行云卡片链接，触发完整 Pipeline：

1. `parseXingyunDemandUrl(link)` 解析链接 → 校验域名白名单、格式、demandId/demandCode
2. SSO 鉴权（`CredentialGuard.ensureValid()`）→ 未登录 → 提示 `poria auth login`
3. `XingyunChannel.getDemandById(demandId)` 获取需求元数据
4. `listCardAttachments(demandCode)` + `resolvePrdFromAttachments()` 解析 PRD 链接
5. 创建 Pipeline 实例（`createPipelineId()` 生成 ID，写入 SQLite）
6. 入队（`PipelineQueue.enqueue(pipelineId)`）
7. 打印 Pipeline ID + 状态提示

**错误处理**：
- 链接格式无效 → 具体错误信息（域名/格式/缺 demandId）
- 未登录 → `AuthRequiredError: 请先运行 poria auth login`
- 需求不存在/已删除 → `需求不存在或已删除 (demandId: xxx)`
- 认证失效 → `登录态已失效，请重新登录`
- PRD 链接歧义 → `PrdResolveError: 需求关联了多个 PRD，请指定`
- PRD 缺失 → `需求未关联 PRD 文档`

#### R1.2: `poria pipeline status <id>`

- 展示 Pipeline 整体状态（created/running/waiting_merge/blocked/completed/failed/cancelled）
- 逐 Stage 展示：名称、状态、耗时、重试次数
- blocked 时展示 issueClass + 等待对象
- waiting_merge 时展示 MR 链接 + 已等待时长

#### R1.3: `poria pipeline list`

- 列出所有 Pipeline（分页，默认最近 20 条）
- 支持按状态过滤（`--status running`）
- 表格展示：ID、需求名、状态、创建时间、耗时

#### R1.4: `poria pipeline resume <id>`

- 从 BLOCKED 状态恢复 Pipeline 执行
- 校验当前状态必须是 BLOCKED
- 标记 blocked stage 为 pending → 重新入队

#### R1.5: `poria pipeline cancel <id>`

- 取消 Pipeline（Design F11）
- 将所有 pending stage 标记为 skipped
- Pipeline 状态 → cancelled
- 执行回滚（调用 `PipelineRollback.execute()`）
- 已 completed/cancelled 的 Pipeline 不可取消

#### R1.6: `poria pipeline rollback <id>`

- 手动触发回滚
- 按 Stage 逆序执行 rollback 指令（close_mr → delete_branch → remove_worktree）
- MR 已合并 → 创建 revert MR + 京ME 通知
- MR 未合并 → 关闭 MR + 删除分支 + 清理 worktree
- 每步幂等（Design F-10：已关闭的 MR 跳过，已删除的分支跳过）

#### R1.7: `poria pipeline replay <id>`

- 回放 Pipeline 事件日志（时间线视图）
- 先查 SQLite events 表，无数据 → 查归档 JSONL
- 按 seq 顺序展示事件摘要

### R2: auth 命令组

#### R2.1: `poria auth login`

- 从浏览器提取 SSO cookie，保存到 `~/.poria/auth.json`
- 保存后调用 `CredentialGuard.ensureValid()` 验证有效性
- 成功 → 打印用户身份（erp）

#### R2.2: `poria auth logout`

- 删除 `~/.poria/auth.json`
- 打印确认信息

#### R2.3: `poria auth status`

- 读取本地凭证 → 调用轻量 API 探测有效性
- 有效 → 打印用户身份 + 过期提示
- 无效/缺失 → 提示登录

### R3: workspace 命令组

#### R3.1: `poria workspace enter <project-dir>`

- 激活项目工作目录
- 检查 git worktree 状态
- 设置当前工作上下文

#### R3.2: `poria workspace exit`

- 退出当前工作上下文
- 清理临时状态

#### R3.3: `poria workspace status`

- 展示当前 worktree 信息：分支、基准分支、变更文件数

### R4: project 命令组（P1 骨架）

#### R4.1: `poria project create`
#### R4.2: `poria project list`
#### R4.3: `poria project status`

P1 仅创建命令骨架（注册 + help text），具体逻辑 P2 实现。

---

## 约束

- 每个命令实现 `ICommand` 契约接口
- 命令通过 `IPluginLoader` 注册和发现
- 所有用户输入做安全校验（链接白名单、ID 格式）
- 错误信息面向用户（中文，明确可操作的建议）
- fixture 模式下所有 channel 调用返回内置 mock 数据

---

## 非目标

- GUI / TUI 交互（纯 CLI stdout）
- 命令自动补全（P2）
- 国际化（固定中文输出）

---

## Acceptance Criteria

- [ ] `poria pipeline submit <valid-link>` 在 fixture 模式下创建 Pipeline 并入队，打印 Pipeline ID
- [ ] `poria pipeline submit <invalid-link>` 输出具体错误信息（域名/格式/缺 demandId）
- [ ] `poria pipeline submit` 未登录时提示 `poria auth login`
- [ ] `poria pipeline status <id>` 展示 Pipeline + 各 Stage 状态
- [ ] `poria pipeline status <nonexistent-id>` 输出 "Pipeline 不存在"
- [ ] `poria pipeline cancel <id>` 将 RUNNING Pipeline 转为 CANCELLED，pending stages → SKIPPED
- [ ] `poria pipeline rollback <id>` 逆序执行回滚指令，每步幂等
- [ ] `poria auth login` 保存凭证，`CredentialGuard` 验证通过
- [ ] `poria auth status` 在有效/无效/缺失三种状态下输出正确提示
- [ ] 全部命令 TypeScript 类型检查通过
- [ ] 全部命令有 unit test，覆盖正常路径 + 主要错误路径
