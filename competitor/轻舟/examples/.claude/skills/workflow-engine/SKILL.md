---
name: workflow-engine
description: "PRD → 上线全链路工作流编排器。子命令：init 建任务骨架 / status 看进度 / next 提示下一步该跑哪个节点。节点编排由 profile 决定（引擎只读节点字段、不认节点名，可自由编排）；编排内节点是 .workflow/node/<名>/NODE.md 文档、CLI 打印斜杠别名 /workflow-engine node <名>（内部代指读该 NODE.md）；转场按闸门分档（auto 自动衔接 / confirm 一键确认 / 重思考与上线节点钉死人工闸）。"
---
# workflow-engine：任务工作流编排

如果发现用户没有走该 profile 的起步节点（`start_node`，如 feature 流的 workflow-explore），直接走了这里，拒绝执行。提示用户先走起步节点。

你的职责就是负责和内部python脚本进行交互帮助用户创建目录，初始化配置等偏机械功能。

> **实现在 `.workflow/engine/`**（`cli.py` + `_lib/` + 分层文档）——引擎是 agent 中立的运行时，与它读写的 `.workflow/` 状态住在一起，不随本 skill 走。本文是入口，机制细节全在那边。

跨节点执行协议（enter/advance/闸门/issue/pending_gate/无人区）的单一源是 **`.workflow/engine/PROTOCOL.md`**，本文只讲 workflow-engine 自己的命令操作，机制细节指向它。

## 用户命令
| 命令 | 行为 |
|---|---|
| `/workflow-engine init <task>` | 建 `delivery/<task>/` 骨架 + 初始 state.json。**执行前先 Read `.workflow/engine/INIT.md`**（profile + 环境的收集流程） |
| `/workflow-engine status <task>` | 打印当前节点 / profile / open feedback / 待兑现闸门 |
| `/workflow-engine roles <task>` | 打印本任务产物区域清单（role → 解析后路径） |
| `/workflow-engine list-profiles` | 打印项目级 `.workflow/profile/` 所有编排模式编号列表；供 onboarding 转贴给用户选，无需 task |
| `/workflow-engine next <task>` | 算下一步，打印对应节点的调用指令 |
| `/workflow-engine node <名> <task>` | 文档节点别名：还原「读 .workflow/node/<名>/NODE.md」。直接执行即可 |
| `/workflow-engine run-loop <task>` | 无人区外层驱动器：分类终态，CONTINUE 给出驱动当前节点的指令、余皆终止打报告。**执行前先 Read `.workflow/engine/RUN-LOOP.md`**（自治/闸门降级/终态分类的完整机制） |

内部直调 `python3 .workflow/engine/cli.py <cmd> <task>`，输出直出给用户、无需二次加工。status / roles / list-profiles / next 是薄查询，直调即可、无需读附加文件。

### profile：编排模式

- `init --profile <名>` 选一份 profile（`.workflow/profile/<名>.json`，每文件一份完全独立的编排，profile 名=文件名 stem）。不带 `--profile`：tty 走向导选、默认 `default_profile`；非 tty（AI 编排）会 loud fail 逼显式带上，不静默默认。列全部**启用**模式跑 `list-profiles`。
- **各档位适用场景与节点组合以 profile 的 `desc` 为准**（`list-profiles` 打印），本文不列举以免漂移。default 为 `develop-mode-openspec`（研发档）。
- profile 机制（来源/enabled/校验/冻结）、`start_node`（起步节点）、`roles`（产物区域）、`context.md`（起步入口）的完整语义见 **`.workflow/engine/PROTOCOL.md`**「项目配置 / 产物区域 / 起步入口 / 起步节点」各段；profile 与节点字段 schema 见 `.workflow/config.json` 的 `_doc_profiles`。选 profile + 起步由 `workflow-start` 承接。

## 出口
在用户没有明确下达允许该环节继续的情况下，不得直接继续。