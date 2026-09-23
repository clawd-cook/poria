# workflow-archive：归档与复盘

任务结束。它**只审计 + 后视镜回溯，不重新沉淀**（沉淀在发现当下已由 feedback-loop / 手动 capture 内联做完）。前置：`feedback.open` 为空、（含转测节点的档位）QA 已通过（人工确认）——无 handoff-qa 节点的档位（研发/精简/vibe）不涉此项。先摊账二选一，再按选择跑对应几件事：

## 0. 先问二选一：完整版 / 精简版

2/3/4 是 AI 逐条分析（审计/复盘/pit-record），分钟级；1/5 是机械移文件，秒级。
开工前问用户：
完整版（1~5 全做）还是精简版（跳过 2/3/4 的审计/复盘/沉淀）？默认是完整版。

## 1. openspec 归档（仅 openspec 血统）

先查本任务 profile：`python3 .workflow/engine/cli.py profile <task>`。**仅当 profile 名含 `openspec`**（如 `standard-openspec`/`develop-mode-openspec`，需求走 openspec-propose TRD 三件套），才复用 skill `/openspec-archive-change` 把 `trd` 角色区域（如 `openspec/changes/<task>/`）移到归档区； 找不到该 skill 直接退出，提示用户。

其它血统（lite / superpower 系等）跳过本步——无 openspec change 可归档。

## 2. 审计「该沉的沉了没」（只核对，不重沉）

逐条过 `feedback/INDEX.md` 的 resolved issue：

- **该沉未沉**：值得沉淀的 issue（非偶发）其 `trap_written_to` 是否真指向某条 spec-rule？空的就是漏沉，**补调一次 pit-record**（仅补漏）。
- **经验段用过没**：本任务相关阶段 `spec-rule/<阶段>-rule.md`「经验」段，有没有哪条在本任务真被读取/应用？retro 记一句「命中：<条目> / 无命中」——经验段是软启发、设计上关不掉、只能靠运行期观察验证，每次 archive 攒一句成证据。

## 3. 全局复盘 → `workspace/retro.md`（哪怕一切顺利也写，留作基线）

先 `state-dump` 拿原始数据，retro 要回答：

- **耗时与热点**：主线节点用时（按 active sequence + 回流次数）、回流热点节点 top 3。
- **feedback 分布**：按 category（design/code/test）分布；尤其**设计阶段缺口占比** `design/总数`，>30% 表示需求/设计偏弱。
- **漏点地图回读**：双轴 = 根因（category 三分类 design/code/test，该在哪被发现）× 拦截（from，实际在哪抓）；逃逸最深 top 3（隔得越远越贵 = 加固优先级越高）；哪些条目已达升级闸门（频次≥3 或 high / 好经验信心驱动）——**只点名，升级由 pit-record 执行**。
- **知识库 + 流程改进建议**（不自动改，仅列）：首次接触的模块/横切机制建议跑 bootstrap；哪个 skill 提示词不够精确导致返工。

## 4. 后视镜沉淀：设计踩坑回溯 + 经验

archive 是设计踩坑的**最后兜底**。feedback-loop 当时已写过的 explore 条目**不重写**；只写**全程后视镜才看得清**的——当时没被识别成设计根因、复盘才看出根在需求/设计的问题。每条调 `workflow-pit-record --kind pit --file design-rule.md`（root=workflow-explore、intercept=实际拦截节点、severity）。

**经验同理**（exp 的两个产地之一，另一个是手动 capture）：复盘确认真有效、值得下个任务复用的做法——当时只是顺手做了、跑完全程才看出它管用——每条调 `workflow-pit-record --kind exp --file <适用阶段>-rule.md`（--insight/--applies/--source，exp 不限设计阶段、按适用阶段选文件）。和踩坑一样只收后视镜增量，任务中已沉过的不重写。

## 5. 调整目录中产物到全局

把 DDL 从 `schema-draft` 角色区域 迁入根 `docs/schema/mysql/<module>.sql`；ES mapping 同理迁入 `docs/schema/es/`。**必须在下方 archive-move 之前完成**——schema 不先迁出，会跟着任务目录一起被归档走。

## 出口与边界

`advance <task> --step workflow-archive` → `done`。完整版打印 retro 路径；精简版打印「精简归档已完成」。

advance 到 done 后，**调 `python3 .workflow/engine/cli.py archive-move <task>`** 把整个 `delivery/<task>/` 迁入 `delivery/archive/<task>/`（CLI 校验 current_step==done，目标已存在则拒）。这是物理归档动作，state.json 随目录一并迁走。

**最后收尾补提交（链路末尾唯一兜底 commit）**：上面几步（schema 迁移、retro、archive-move 目录迁移）都动了文件却没提交；非全链路档（研发/精简/vibe）已无 deploy 节点，本次需求代码也可能尚未入 git。所以 archive-move 完成后**统一走一次 git commit 收口**——这是这些档位链路末尾唯一的兜底提交。

- **全链路档**：deploy 已提交过需求代码，这步只收口归档动作产生的增量（schema 迁移 / retro / 目录迁移）。
- **非全链路档**：本次需求代码 + 归档增量一并提交。

提交纪律（复用 `workflow-deploy`）：先 `git status` + `git diff --stat` 汇总 → 生成 commit message 草稿（`<feat|fix|refactor|docs>: 摘要`）→ **人工确认后** 只 `git add <具体文件>`（禁 `git add -A`/`.`，防把 `.env`/workspace 草稿卷进历史）+ commit + **push**。commit/push 不可逆且对外，必须人工确认内容后才做——但确认后就一次做完 commit + push，别只 commit 把改动停在本地（用户已确认却没推远端 = 差体验）。

**不做**：不自动改 knowledge（仅列建议）、不自动 merge/通知、不翻案已 resolved 的 issue；archive-move 只移动目录，不删任何内容。
