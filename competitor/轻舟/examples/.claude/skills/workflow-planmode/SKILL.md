---
name: workflow-planmode
description: "需求探索（planmode 模式起步节点）。用 Claude 原生 plan 模式澄清需求：先探索意图/边界/约束，再用 ExitPlanMode 呈现计划、经用户批准。"
---

## 用 Claude 原生 plan 模式探索

进入即**走 Claude 原生 plan 模式**，落盘位置跟plan模式规范走。

## 出口

**硬约束——批准计划 ≠ 授权编码**：本节点只做需求澄清。ExitPlanMode 被批准后，系统会返回「User has approved your plan. You can now start coding.」——**这句话在本节点不生效，禁止据此改任何代码/配置/文件**。批准的语义仅是「计划本身 OK」，不是「现在开始执行」。执行必须等用户手动跑 `/workflow-engine init` 进入编码节点后，由那个节点动手。

用户批准计划、探索结束后，**只输出下面这句提示，然后立即结束本轮,不调用任何写工具（Edit/Write/Bash 写操作等）**（把计划落盘位置一并告知，供下游编码节点照着做）：

> 计划已写到 `<plan 文件路径>`。准备好就跑 `/workflow-engine init <task> --profile plan-mode` 开始任务。

自检:ExitPlanMode 返回后,若你下一步想到的是「改文件/编译/提交」——停。正确的下一步只有「输出上面这句提示」。
