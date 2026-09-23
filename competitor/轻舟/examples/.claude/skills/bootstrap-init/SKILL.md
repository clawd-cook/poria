---
name: bootstrap-init
description: "AI 一键初始化项目知识底座。锚定 project-overview + tech-topic 后停顿等用户 review，确认后各生成默认 2 个（可调）module/tech 文档（子 Agent 并行），再补 material-map。"
---

# bootstrap-init：AI 一键初始化知识底座

一次调用把项目从「`docs/` 全是示范填充」带到「真实知识底座落地、workflow 任务流能加载真实上下文」——**本窗口自己编排、自己跑**，不把活推给用户去敲 CLI。

核心哲学：**磁盘产物是进度真值，`.workflow/bootstrap-state.json` 是给框架其它环节看的公共账本**——两者都查、两者都维护，已落地的跳过，天然幂等可重跑。

---

## 进度账本：`.workflow/bootstrap-state.json`（用脚本读写）

框架其它环节会读它判断「知识底座建到哪了」，所以**必须维护**。但**绝不手写这份 JSON**。改用本 skill 目录下的确定性脚本 `state.py`。下文用 `STATE` 代指 `python3 .claude/skills/bootstrap-init/state.py`：

```
STATE mark  overview            # 标记某步完成（自动建中间对象、自动先迁移旧文件、盖时间戳）
STATE mark  tech_topic          # 注意：Phase A 的挖掘步（扁平键，带下划线）
STATE mark  modules.1           # 点路径：第 1 个业务模块
STATE mark  tech.2              # 点路径：第 2 个 tech 文档（≠ tech_topic）
STATE mark  material_map
STATE done  overview            # 查询：完成打印 done 退出码 0，否则 not-done 退出码 1
STATE show                       # 打印整份 state（不存在打印 {}）
```

> 别混淆两个 tech 键：`tech_topic`（扁平、带下划线）是 Phase A「挖掘 topic 并回填 overview 索引」那一步的完成态；`tech.N`（点路径、嵌套）是 Phase B 里第 N 个 tech 文档细化的完成态。两者独立。

schema（脚本产出，仅供你判读，别照着手写）：

```json
{
  "overview":     { "done": true, "at": "2026-07-27 14:30:00" },
  "tech_topic":   { "done": true, "at": "2026-07-27 14:35:00" },
  "modules":      { "1": { "done": true, "at": "..." }, "2": { "done": true, "at": "..." } },
  "tech":         { "1": { "done": true, "at": "..." } },
  "material_map": { "done": true, "at": "..." }
}
```

- key 语义：`overview` / `tech_topic` / `material_map` 顶层布尔态；`modules.<序号>` / `tech.<序号>` 按生成的第几个记（序号从 1 起）。
- **判进度＝`STATE done <key>` ＋ 读盘双确认**：脚本说 done 但对应产物文件已不在（被手删）→ 当没做、重新生成。别只信 state，也别只信盘。
- **`STATE mark` 的时机＝产物确实落地之后**，由**本主窗口**调用，不交给子 Agent 调（多 Agent 并发写同一 json 会 race）。
- 旧位置 `.claude/bootstrap-state.json` 的迁移由脚本自动处理（`mark`/`done`/`show` 前都会先迁），无需你操心。

---

## Phase A · 锚定（子 Agent 串行，**不用主窗口生成**）

overview 是「锚」，后面所有填充都挂在它的索引上，物理上必须先落；tech-topic 依赖它——它是**开一趟独立上下文、专项深挖项目亮点级横切技术设计**的活（不是顺手填索引：回填 overview 触发式索引、把占位 `涉及xxx` 换成真实 topic 只是这趟深挖的副产物）。所以这两步**串行**：overview 完再 tech-topic，overview 失败直接终止、不进 tech-topic。

两步**都丢给子 Agent 跑，不占主窗口**：overview/tech-topic 都要扫全项目代码、上下文极重，放主窗口会撑爆。**主窗口全程只当编排器**：派 Agent、读产物判进度、`STATE mark`。

1. **project-overview**
   - 已落地（`docs/project-overview.md` 存在且不含「本文为示范填充」）→ 跳过；`STATE done overview` 若返回 not-done 则补 `STATE mark overview`。
   - 否则起一个子 Agent 跑 `/bootstrap-project-overview` 扫码落盘。Agent 返回后主窗口确认产物存在 → `STATE mark overview`。文件没生成 → 终止报错。
2. **tech-topic**（overview 子 Agent 成功返回后才起，串行）
   - 判跳过只认：`STATE done tech_topic` 返回 done ＋ 深挖草稿 `docs/tech-knowledge/_topic-notes.md` 存在——**两者都满足才跳过**。`/bootstrap-tech-topic` 是单独上下文专项深入挖掘技术问题的技能，不可拿 overview 随便写的 topic 替代，那种深度太浅。
   - 否则起一个子 Agent 跑 `/bootstrap-tech-topic` 通读挖掘、回填 overview 触发式索引 + 留 `docs/tech-knowledge/_topic-notes.md` 挖掘草稿。Agent 返回后 → `STATE mark tech_topic`。

---

## 闸门 · Phase A 完 → 停下等用户 review（**不自动进 Phase B**）

overview 是锚、tech-topic 索引是后面所有填充的挂载点——**锚错了，Phase B 并行生成的一堆文档全跟着错**。所以 Phase A 落盘后**必须硬停**，把成果摆给用户看、等反馈，别默认往下冲。

1. **摆成果**（只给位置、不打印内容）：告诉用户 overview 已落盘在 `docs/project-overview.md`，请自己打开过目——重点看正文，以及里面的**触发式索引**（module 业务模块索引 + tech topic 索引，那是 Phase B 决定按哪几条 fan-out 生成文档的依据）。主窗口不打印 overview 全文、也不摘录索引条目。
2. **等 review**：请用户过目 overview 内容与索引条目——是否准确、有无该删该改该补的。**用户明确认可前，不派 Phase B 任何 Agent。**
   - 用户提修改 → 就地改（或重派对应锚定 Agent 重跑），改完再摆一次、再等确认。
   - 用户认可 → 进下面「确认生成数量」。
3. **确认生成数量**（顺带在同一轮问，省一次往返）：告诉用户 module/tech **默认各生成 2 个**，问要不要调整（可各自指定，如 module 3 个、tech 1 个；也可直接用默认）。把用户给的数记为 `N_module` / `N_tech`，缺省都是 2。索引条目不足所要数量时按实际条目数封顶、不硬凑。

> 这个闸门是「锚定 review」，只在 overview/tech-topic 之后停一次；Phase B 内部并行生成不再逐个停。

---

## Phase B · 填充（子 Agent 并行 fan-out）

**生成数量由上面闸门确认**：module 生成 `N_module` 个、tech 生成 `N_tech` 个（用户没特别说＝各 2 个）。module/tech 是非交互纯生成任务、且互不依赖，正好一人一个子 Agent 并行，独立上下文不污染主窗口。

**派活规则**（用序号派、不在文本里静态解析模块名——AI 输出格式会飘）：

- **module ×`N_module`**：对第 1 … 第 `N_module` 个业务模块，各起一个 Task 子 Agent：
  > 基于 `docs/project-overview.md` 业务模块索引中的**第 N 个**业务模块，执行 `/bootstrap-module-knowledge` 生成文档。
- **tech ×`N_tech`**：对第 1 … 第 `N_tech` 个 tech topic，各起一个 Task 子 Agent：
  > 基于 `docs/project-overview.md` 触发式索引中的**第 N 个** tech topic，执行 `/bootstrap-tech-knowledge` 细化生成文档。若该索引不足第 N 个（仍是占位或条目不够），直接跳过、不要硬凑。
- **material-map**：起一个子 Agent 跑 `/bootstrap-material-map`（恒随批生成，值不值得由该 skill 自评）。

**并发方式**：在**一条消息里发多个 Task 调用**让它们同时跑。

**幂等**：派活前先查——对应产物已落地（`docs/module-knowledge/<模块>.md` / `tech-knowledge/<主题>.md` / `docs/test-knowledge/物料地图*.md` 已存在，或 `STATE done modules.N`/`tech.N`/`material_map` 返回 done 且文件在）→ 跳过那个，不重复起 Agent。

**收 state（每完成一个立刻写一个）**：子 Agent 只负责生成文档、回报成功/失败，**不碰 state**；**主窗口每收到一个成功返回，就立刻对它 `STATE mark`**（`STATE mark modules.1` / `STATE mark tech.2` / `STATE mark material_map`），不攒批。脚本是串行原子写，主窗口逐个调即可，不会互相覆盖。失败的不 mark、如实报告，重跑只补没成的。

**overview 写 race**：module 和 tech 生成收尾都会回写 overview 索引，多 Agent 并发写同一文件有 race——各窗口只动自己那段，接受之。

---

## 收尾

汇总本次产出，一目了然：
- `docs/project-overview.md`
- `docs/module-knowledge/*.md`（列已生成的）
- `docs/tech-knowledge/*.md`（列已生成的，排除 README / `_topic-notes` / Overview）
- `docs/test-knowledge/物料地图*.md`
- 进度账本：`.workflow/bootstrap-state.json`

若有子 Agent 失败或想生成更多（默认各 2 个、或上面闸门里用户指定的数量），提示：直接再调一次本 skill 补缺（已落地的会跳过），或用户点名要第 3、第 4 个。

---

## 负空间（别越界）

- **主窗口只编排、不生成**——所有文档生成（含锚定的 overview/tech-topic）都丢子 Agent 隔离上下文；主窗口只派活、读产物判进度、写 state。overview→tech-topic 串行（锚 + 索引依赖），**Phase A 完硬停等用户 review 锚定成果 + 确认生成数量**，认可后 module/tech/material 才并行 fan-out。
- **state 一律走 `state.py` 脚本、禁手写 JSON**——由主窗口在每个产物落地后立刻调 `STATE mark`，不让子 Agent 调、不攒批、不手搓那份 json（键/嵌套/时间戳漂移即污染公共契约）。
- **不强求填满**——「值不值得写」是各 `/bootstrap-*` 子 skill 的价值闸；tech 不足自动跳过，本 skill 只排序 + 派活，不硬凑。
- **不替各子 skill 改内容规范**——本 skill 只编排调用它们。
