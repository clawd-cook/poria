<!-- LIGHTBOAT:NAV:BEGIN 本区块由 lb init 自动维护，请勿手改；改动会在下次 init 被覆盖 -->
# 仓库导航

本仓库是「PRD → TRD → Test → CR」全链路 AI 协作骨架。
**本文件是地图**：定位 + 路由指针；机制细节在对应 skill，本文只挂指针不复述。

## 目录骨架（写产物前先看）

| 顶层 | 放什么 |
|---|---|
| `delivery/<task>/` | 任务产物（state.json / prd / prompt / types / test / review / feedback / workspace ...） |
| `openspec/` | 开发规范与技术方案（TRD 等） |
| `docs/` | 长期知识库（`project-overview.md` / `spec-rule/` / `*-knowledge/`） |
| `scripts/` | 工程脚本（`lint/` 架构约束、`test/` 自动化测试） |

**完整目录树、各文件用途见 `.workflow/workflow-map.md`。**

## 写入规则

1. 任务产物 → `delivery/<task>/`；**不确定放哪一律进它，别新建未列出的目录**。
2. TRD / 技术方案 → `openspec/`。
3. **学习层豁免**：spec-rule 三段在任务中**内联写** `docs/spec-rule/<阶段>-rule.md`（由 `workflow-pit-record` 统一写）——规则 1 只管任务产物，学习层一直内联写。
4. 沉淀分诊（规定性/陈述性该进哪）见下「手动 capture」。
5. CR 阶段规约独立在 `docs/spec-rule/code-review-rule.md`（团队声明的「过闸必查」检查项 + CR 退出时半自动沉淀风格偏好，见 `workflow-code-review` 节点）；框架基线评审维度仍内联在节点文档（`.workflow/node/workflow-code-review/NODE.md`）。

## 手动 capture（你说「记一下：…」）

任意时刻人主动要求沉淀，按一个问题分诊：**是在纠错/约束一个做法（规定性 ought），还是陈述一个事实（陈述性 is）？**

- 规定性（踩坑/经验）→ spec-rule 对应段，写手 `workflow-pit-record`
- 陈述性（模块/技术长啥样）→ `module-knowledge/` / `tech-knowledge/`，直写
- 本次取舍 → `delivery/<task>/workspace/`，直写

**红线**：规定性进 spec-rule，陈述性进 knowledge，放错书架=检索失败。人主动说「记下来」已替你判过「值得」，跳过闸口。若记的是「还没修的问题」→ 走 `/workflow-feedback-loop <task> --new`。字段格式与升级闸门见 `workflow-pit-record` skill。

## 学习层（spec-rule 三段制）

每份 `docs/spec-rule/<阶段>-rule.md` 含三段：**规约**（必须遵守）/ **经验**（阶段性启发）/ **踩坑**（犯过的错），统一由 `workflow-pit-record` 写。条目字段格式、漏点地图双轴、升级闸门的完整定义都在 **`workflow-pit-record` skill**，本文不复述。

## 阶段读取清单

读 spec-rule 时：**规约**必须遵守；**经验**段开工前扫一遍挑相关的（软启发、易被略读，需主动扫）；**踩坑**段避坑。

| 阶段 | 动作          | 先读 |
|---|-------------|---|
| 任意 | 进入任务前       | `docs/project-overview.md` |
| TRD | 需求探讨时/写技术方案 | `design-rule.md` + `tech-knowledge/<相关主题>` + `module-knowledge/<相关模块>` |
| 编码 | 写代码         | `code-rule.md` + `module-knowledge/<相关模块>` + `tech-knowledge/<相关主题>` |
| 编码(UI) | 写 UI 组件/样式 | 上述 + `.workflow/node/workflow-implement/ui-protocol.md`（主 agent 规则 + 骨架/静态稿/组件三段 subagent 规则：px 配置对齐、图层→DOM 转换、属性归属边界） |
| 测试 | 写测试         | `test-rule.md` + `test-knowledge/` |
| CR | 评审          | `code-rule.md` + `design-rule.md` + `code-review-rule.md`（团队 CR 检查项；框架基线评审维度内联在 workflow-code-review 节点文档） |

## 内部文档读取

Joyspace 链接（`joyspace.jd.com`）→ `lbcli joyspace read <URL>`，禁用 WebFetch。

## 工作流编排（workflow）

**起步正门**：拿到需求先跑 `/workflow-start`——列出可用模式让用户选，衔接该模式的起步节点开始澄清。

起步/触发节点（`start_node`，如 `/workflow-explore`、`/workflow-bugfix`）是工作流正门：先把需求聊清楚，它**在状态机外、init 之前跑**，刻意不加载下方编排细节（分阶段加载）。

澄清结束后由人手动跑 `/workflow-engine init` 进入工作流。任务级流程由 `/workflow-engine` skill 编排（`init` 建骨架 / `status` 看状态 / `next` 提示下一步 / `roles` 查产物路径 / `list-profiles` 列模式）。节点公共协议先读 `.workflow/engine/PROTOCOL.md`。

- **编排靠 profile**：节点组合/顺序/闸门由 `.workflow/profile/<名>.json` 定义（一文件一 profile）。**各档位适用场景与节点组合以 profile 的 `desc` 为准**（`list-profiles` 打印），本文不复述以免漂移。前端项目 init 时 `frontend/_workflow/` 同名覆盖共享 profile（如 `standard-openspec` 前端版无 deploy 节点）。
- **转场闸门**分档（auto / confirm / confirm-locked；重思考与上线节点钉死 confirm-locked），auto 请直接执行、不要询问。
- **反馈回路**、闸门表、state.json schema、CLI 子命令**全在 `workflow-engine` skill**，本文只留路由。核心价值观：验证型节点记下的问题**默认原地修**、用户主动要求才回退。
- 编排内节点是 `.workflow/node/<节点>/NODE.md` 文档（非 `/` skill）：进入时先读 `.workflow/engine/PROTOCOL.md`（本窗口首次）+ 调 CLI `enter` 过门禁，放行后 Read 该节点 NODE.md 开工，产物落对应子目录后由 CLI 更新 state.json。CLI 转场提示打斜杠别名 `/workflow-engine node <节点>`（内部代指「读该 NODE.md」）即指令。

## codegraph 使用约束（控制上下文膨胀）

- **上下文经济**：每个塞进上下文的 token 必须有价值。`Read` 大文件用 `offset`/`limit`；工具调用前先问「返回里多少比例是这一步真要用的」，比例低就换更窄的工具。
- **codegraph 金字塔（轻→重）**：`search`（找位置）→ `node`（默认 `includeCode=false`）→ `trace`（窄路径，限定 from/to）→ `context`/explore（宽包，重型）。起点锚要小（`context` 起手 `maxNodes=8` + `includeCode=false`），不够先停下来想再加，别直接拉默认值。
- **触发反思 / 禁止**：一次 query 要写 ≥5 个名字 → 先 search 看分布；连续调用 ≥3 次未定位 → 换思路；想「再补一点以防万一」→ 不补。已显示过的文件不再 Read；同一符号一次任务内不重复 `includeCode=true`；纯文档/配置文件任务不调用 codegraph。
<!-- LIGHTBOAT:NAV:END -->
