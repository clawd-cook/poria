# 全链路对话式交付编排

## Goal

把 Poria 从「固定六阶段静默跑完」改为对齐轻舟 **standard-openspec 全链路**的交付编排；阶段**内部**仍可用非交互执行，阶段**边界**必须经工作台对话确认后才 `advance`。禁止整条流水线静默 autopilot。

用户价值：全链路可见、关键闸可对话确认、仍交付到 Coding MR。

## Decisions（已确认）

1. **范围 = 全链路**（对标 `standard-openspec`）：clarify/propose → test-plan → implement → lint → code-review → test-cases → run-autotest → handoff_qa → deploy → archive；保留 Poria **Init**（工作区/文档导出）与强化 **Deploy**（push + EasyCI + MR）。
2. **推进 = 开对话，不静默全跑**：禁止跨阶段 `run-loop` autopilot。
3. **对话深度 = 方案 A（阶段边界对话）**：阶段内可用现有 `claude -p` / 机械命令；阶段结束后系统在对话切面给摘要与产物入口，用户回复「继续 / 重做 / 跳过 / 补充」后再进入下一阶段。本任务不做阶段内多轮续聊（方案 B）。

## Background

- 竞品：`competitor/轻舟/examples/.workflow/profile/standard-openspec.json`、`workflow-map.md`。
- 工作台壳已交付（归档 `09-23-demand-workbench-204`）；本任务改编排与边界对话。
- 今日：`Init → ReviewPrd → Design → Dev → Cr → Deploy`，Blocked 才 HITL。

## Requirements

| ID | Requirement |
|----|-------------|
| R1 | `poria-core` 定义 **full profile** 节点序（可版本化）；旧六阶段可映射迁移。 |
| R2 | 执行器默认 **每节点结束后停止**，发 `awaiting_advance`（或等价）对话请求；无用户确认不得进下一节点。 |
| R3 | 工作台 **对话切面**展示：阶段完成摘要、产物链接、可选操作（继续/重做/跳过/补充上下文写入下一阶段 prompt）。 |
| R4 | **confirm-locked** 至少：clarify/propose、test_plan、run_autotest、handoff_qa、deploy、archive。 |
| R5 | lint 失败停在对话并说明，不静默跳过。 |
| R6 | CR / 测试失败可建 feedback issue；对话中选择回 implement 或确认带病继续（策略显式）。 |
| R7 | 测试相关节点 MVP：以文档产物 + 可选本地命令为主，不强制 Playwright/轻舟 engine。 |
| R8 | 阶段内保持非交互 agent（`-p`）；补充上下文仅影响**下一阶段**入参，不打开阶段内多轮 session。 |
| R9 | 桌面窗验收；未登录不打行云。 |

## Acceptance Criteria

- [ ] AC1：full profile 阶段在工作台可见，顺序覆盖上表全链路（含 Init/Deploy/Archive）。
- [ ] AC2：Init 完成后不自动连跑到 Deploy；每阶段结束进入对话等待。
- [ ] AC3：对话「继续」才 advance；「重做」重跑当前节点；「跳过」仅允许非 locked 节点或显式高风险确认。
- [ ] AC4：propose 与 test_plan 无确认不得进入 implement。
- [ ] AC5：相关 `cargo test` / `pnpm typecheck` 通过；桌面手测停—聊—继续。

## Out of Scope

- 阶段内多轮「继续会话」agent（方案 B）
- OpenSpec CLI / Relay / 视觉像素门 / 云终端整包移植
- 静默跨阶段 autopilot

## Technical Notes

- 详见 `design.md`、`implement.md`。
- Spec 清单：`implement.jsonl` / `check.jsonl`。

## References

- `competitor/轻舟/examples/.workflow/`
- `docs/versions/2.0.4/PRD.md`
- `crates/poria-core` / `poria-commands` / `poria-skills` / `src/components/DemandWorkbench.tsx`
