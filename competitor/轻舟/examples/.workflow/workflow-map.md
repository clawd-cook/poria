# 工作流目录地图（产物落盘详情 · 前端）

本仓库是「PRD → TRD → Test → CR」全链路 AI 协作骨架。CLAUDE.md 里有一级骨架 + 写入规则，这里是细粒度子目录说明——写产物拿不准具体落哪个子目录时来查。

## 目录地图

```
<project-root>/
├── CLAUDE.md                              # 项目入口导航（始终加载）
├── .workflow/                             # 工作流引擎配置（非 skill；由 lb init 覆盖下发）
│   ├── config.json                        # 项目级不变量（app_info / default_profile 等）
│   ├── profile/                           # 编排模式（一文件一 profile，节点集/顺序/闸门）
│   ├── node/                              # 编排内节点的执行文档：<节点>/NODE.md（前端项目含前端覆盖版）
│   │                                      # 由 CLI next/enter/advance 打印斜杠别名「/workflow-engine node <节点>」调起（内部代指读该 NODE.md），不是独立 skill
│   └── workflow-map.md                    # 本文件
├── scripts/                               # 工程脚本
│   ├── lint/                              # 架构约束脚本（模块边界/依赖方向等），编码阶段必须执行
│   └── test/                              # 自动化测试相关脚本
├── openspec/                              # 开发相关规范与技术方案（TRD 等）
├── delivery/                              # 任务交付物（按任务分桶）
│   └── <task>/
│       ├── state.json                     # 编排状态（由 workflow-engine CLI 维护）
│       ├── prd/                           # 原始 PRD 文档
│       ├── prompt/                        # PRD 转化后给 AI 执行的提示词
│       ├── types/                         # 数据结构草稿（API 契约 / TypeScript 类型定义）
│       ├── test/                          # test-plan.md / test-cases.md / test-report.md
│       ├── review/                        # workflow-code-review 每轮一份 findings-<n>.md
│       ├── feedback/                      # workflow-feedback-loop 处理的问题：INDEX.md + issue-<n>.md
│       ├── handoff-report.md              # 转测报告
│       └── workspace/                     # 任务工作区：调研、决策记录、过程笔记、retro.md（archive 复盘）
└── docs/                                  # 长期知识库（跨任务复用）
    ├── project-overview.md                # 项目概览：业务背景 / 技术栈 / 关键约束（始终加载）
    ├── spec-rule/                         # 团队规范（每份含「规约」+「经验」+「踩坑」三段）
    │   ├── design-rule.md                 # 设计 / TRD 阶段（explore/propose/test-plan 共用）
    │   ├── code-rule.md                   # 编码阶段（implement/code-review 共用）
    │   └── test-rule.md                   # 测试阶段（test-plan / test-cases / run-autotest）
    │                                      # CR 阶段无独立 rule：评审维度内联在 workflow-code-review 节点文档
    ├── tech-knowledge/                    # 横切技术决策（如 主题切换方案 / 国际化 / 微前端通信等通用技术设计）
    ├── module-knowledge/                  # 业务模块领域知识（按业务实体一份）
    └── test-knowledge/                    # 测试方法论沉淀
```
