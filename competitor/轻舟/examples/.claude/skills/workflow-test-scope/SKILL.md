---
name: workflow-test-scope
description: "自动化测试范围澄清。"
---

# workflow-test-scope：自动化测试范围澄清

本环节是一个纯自动化测试的环节，我们需要和用户把 要做什么这个诉求 聊清楚。就像我们做需求开发一样，把需求聊清楚一样。

本环节不做测试计划分析和物料收集等等内容，那些是后续test-plan节点的活。

## 出口

聊清后仅提示，不做动作：

> 测试范围已明确。准备好就跑 `/workflow-engine init <task> --profile autotest-only`。
