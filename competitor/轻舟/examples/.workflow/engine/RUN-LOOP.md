# run-loop：无人区自驱

> 敲 `/workflow-engine run-loop <task>` 时按需读本文；其余命令用不到，不必读。

`run-loop` 把无人区串成真 loop。**CLI 是 state 薄壳、不能执行节点本身**，所以 run-loop 不是「Python 跑完整条流水线」，而是**每个节点边界你（AI）调一次的「分类器 + 指令」**——循环体活在对话里。

**自治 = 跑 run-loop 这个动作本身，不固化到任何配置**（无「无人模式」持久开关）。降级能力仍在（`nodes.transition_gate(..., autonomous)`），但只在 run-loop 发出的 `advance --step X --loop` 这一次成立：`--loop` 把非锁定闸降 `auto`（锁定闸 `confirm-locked` 经 `_stricter` 保留，无论出入口）。手动 `advance`（无 `--loop`）照基础闸门走。粒度落在「单次转场」，由「这趟 run-loop」串成需求级无人跑，跑完不残留。

**两套判定解耦**——① **出口钉死**（看闸门值 `confirm-locked`，`resolve_exit_gate` 判）决定转场要不要人点，`--loop` 不降级；② **停人节点**（看冻结 pipeline 每节点的 `autopilot` 布尔，`classify_loop` 判）决定 run-loop 走到该节点驱不驱动——非 `autopilot` 即交还人。`autopilot` 缺省 false（停人），只机械节点在 profile 显式标 true（default profile：run-autotest/implement/lint/unittest/code-review/test-cases 标 true；propose/test-plan/deploy/handoff-qa/archive 走默认停人）。`execution==external-window` 的节点不能标 `autopilot:true`，否则 loud fail。run-autotest 出口钉死但 `autopilot:true`：run-loop 驱动它跑回归+自修，跑完 advance 命中出口 `confirm-locked` 停人确认。

**跨入无人区那一刻要停下来问，不是自己拍板**：`advance`（非 `--loop`）/`init` 落到「刚进无人区」的节点（`nodes.is_zone_entry`）时，CLI 打的不是一行可以无视的建议，而是**强制确认指令**（`run_loop_zone_entry_confirm`）——你必须向用户确认「是否要用 run-loop 自动模式跑完这段」。不要因为节点本身是 auto 就不问，run-loop 更大的意义是问题自修复不打扰用户，这是普通 auto 不具备的。

**你的循环协议**：跑 `run-loop <task>` 看输出分类（纯由 `nodes.classify_loop(state)` 按 state 判定，优先级从上到下）：

| 分类 | 触发 | 你的行为 |
|---|---|---|
| `ESCALATED` | 任一 open issue `reroute_count ≥ 3` | 终止：列现象/历次尝试，升级人工 |
| `HIGH_ISSUE` | 任一 open issue `severity == high` | 终止：跑 feedback-loop 由人处置 |
| `TRIAGE` | 有 open issue（无 high/escalated） | 跑 `/workflow-feedback-loop`（normal 自裁默认原地修+闭环）→ 闭环后再调 run-loop |
| `PINNED` | `current_step` 节点非 `autopilot` | 终止：人认证产物 / 转测·归档后续（deploy 先走节点内上线前置清单逐条确认） |
| `CONTINUE` | 其余（含 run-autotest） | `enter --step <node>` → 干活 → `advance --step <node> --loop` → 再调 `run-loop`。节点边界上下文若已长，先 compact 或把重活派 subagent 取蒸馏结论 |
| `DONE` | `current_step == done` | 终止：交运行报告 |

issue 类优先于钉死节点判定（open issue 本就锁主线，高于任何闸门）。运行报告由 CLI 从 `state.main.history + feedback` 确定性生成（不靠你记账）。
