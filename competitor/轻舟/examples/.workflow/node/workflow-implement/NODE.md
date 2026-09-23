# workflow-implement

先查本任务 profile：`python3 .workflow/engine/cli.py profile <task>`。

- **openspec 血统**（profile 名含 `openspec`，如 `standard-openspec`/`develop-mode-openspec`，需求由 openspec-propose 产 TRD 三件套）：**复用 skill `/openspec-apply-change`**。找不到该 skill 直接退出，提示用户。
- **其它血统**：按具体计划的位置执行编码（找不到就看 `trd` 角色区域）。

## 任务节奏（上下文预算）

推进前按任务内容**预判工作量**，评估是否分多轮 subagent——判据是上下文预算，不是执行速度。**理想单窗口占用 60k–120k**：过大降质量，过少冷启不划算。

## 前端任务

项目类型是前端(Frontend)时，`Read 本目录下 ui-protocol.md`：
- 主 agent 执行「主 agent 规则」段
- spawn subagent 时遵循「subagent 编排」段的路由规则

## 单测

本节点不跑单测。如果当前 profile 包含 `workflow-unittest` 节点，单测在该节点执行；如果 profile 无此节点则忽略。

## 跨仓库改动（兜底闸门）

编码时若目标文件落在当前工作区之外，如果缺少权限的话，发起授权即可，不得因为权限问题漏做需求。

## 回流场景

来自 workflow-feedback-loop 的代码修复：**只改 issue 描述的具体位置，不顺手重构**；在 issue「修复结果」段写改了哪些文件、为什么；修完调 `issue-resolve --by "implement (round <n>)" --mode <auto|asked>`（`--mode` 必填，如果这个问题的解决是ai问过人之后解决的是asked，ai自己修复的就是auto）。

## 出口（流转门）

**代码编译通过**。

后端：DDL/ES mapping 草稿落定 `schema-draft` 角色区域。

回流场景先 `issue-resolve` 再统一 `advance <task> --step workflow-implement`，照 CLI 打印的闸门指令当场接续。
