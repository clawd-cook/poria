# 产品知识库：怎么用

工作目录要在**知识库仓库根**（能看到各产品目录和 `.claude/skills/` 的那一层）。

---

## 你要先准备

- 产品英文目录名（kebab-case，如 `my-product`）
- 相关代码库的 git 地址（SSH：`git@host:org/repo.git`）和各自职责
- 可选：中文名、一句话介绍
- 你对这个产品的补充说明（会写进 `context.md`，见下）

---

## 第一次建库

### 1. 建输入文件

两个输入文件一起建：`codebase.json`（代码库清单）+ `context.md`（你的补充）。跟 AI 大白话说，让它落文件。例如：

> 我要建产品知识库，目录名 my-product，中文名「某某产品」。  
> 代码库两个：[git@coding.jd.com](mailto:git@coding.jd.com):org/repo-a.git 是后端核心，[git@coding.jd.com](mailto:git@coding.jd.com):org/repo-b.git 是 Web 前端（看 develop 分支）。  
> 另外这些背景你写进 context：……  
> 帮我把输入文件建好。

建好后先核对再往下：链接、职责、分支对不对；`context.md` 是否把你想交代的都落进去了。

`codebase.json` 大概长这样（`branch` 不写 = 远端默认分支）：

```json
{
  "product": "my-product",
  "name": "我的产品",
  "description": "一句话说清这个产品是做什么的",
  "repos": [
    { "git": "git@coding.jd.com:org/repo-a.git", "role": "后端核心业务" },
    { "git": "git@coding.jd.com:org/repo-b.git", "role": "Web 前端", "branch": "develop" }
  ]
}
```

- `context.md` **为什么重要：** 可以理解context.md 是你给ai的提示，这样它生成起来会更快，同时效率质量也比较高。


### 2. 生成汇总和能力地图

> 帮我给 my-product 建产品知识库

会产出：

- `product-overview.md`：产品汇总 + 能力地图
- `capability-map.json`：地图的机器可读版

这一步只出骨架，不会把每个域写深。地图读一遍，域划分不对就直接让它改。

### 3. 细化能力域

挑一个马上要用的域：

> 细化 my-product 的「客户管理」能力域

一个域一个域来。文档会进 `my-product/客户管理/`，并回填地图链接。

### 4. 校对纠偏

发现写错或想补背景，直接说：

> my-product 里群分配那段不对，实际是按建群时间取前 10% 再随机，帮我改一下。

判断标准：评审时有人追问这条规则，光看文档能不能答清楚。答不清 = 让它深挖重写。

### 5. 代码更新后同步

> my-product 的代码更新了，帮我增量同步一下知识库

只刷新**已经生成过、且受影响**的文档。没细化的域不会被自动补出来；地图上没有的新能力只会提议，不会擅自建。

---



## 怎么开口


| 想做什么                 | 怎么说                             |
| -------------------- | ------------------------------- |
| 建库 / 加仓库 / 改分支 / 改职责 | 「帮我给 XXX 建输入文件」「给 XXX 加一个前端仓库…」 |
| 第一次出汇总和地图            | 「帮我给 XXX 建产品知识库」                |
| 把某块写深                | 「细化 XXX 的『客户管理』」                |
| 某处写错了 / 补背景          | 「XXX 的 YY 那段不对，实际是…，帮我改」        |
| 代码更新了                | 「XXX 代码更新了，增量同步一下」              |


不用记场景名，大白话即可。

---



## 产出目录

能力域目录名、主文档名用中文，和地图上的域名一致。

```
my-product/
├── codebase.json
├── context.md
├── product-overview.md
├── capability-map.json
└── 客户管理/
    ├── 客户管理.md
    ├── 客户标签.md
    └── 客户分配/
        ├── 分配策略.md
        └── 分配限制.md
```

---



## 常见问题

**拉代码权限失败？**  
先手动 `git clone git@...` 验证本机 SSH。单个库失败不会整单中断。

**没有旧 PRD？**  
不影响。代码是主依据；PRD 和代码冲突时以代码为准。

**想一次写透整个产品？**  
不建议。先地图，再按需细化马上要用的域。

**文档太浅 / 老跑偏？**  
先补 `context.md` 再细化或重跑；单篇不对就指出具体规则让它深挖。

**改产品名 / 加仓库 / 换分支 / 改范围？**  
走「建输入文件」那类诉求（会改 `codebase.json` / `context.md`）。改某段梳理结论走校对纠偏。

---



## 附录：研发 / 工具

产品同学可跳过。

**脚本**（在知识库仓库根执行，`$SKILL_DIR` = 本 skill 目录）：

```bash
python3 "$SKILL_DIR/tools/codebase.py" clone <产品名>
python3 "$SKILL_DIR/tools/codebase.py" list  <产品名>
python3 "$SKILL_DIR/tools/codebase.py" path  <git链接>
python3 "$SKILL_DIR/tools/codebase.py" mark  <产品名>
python3 "$SKILL_DIR/tools/codebase.py" diff  <产品名>
```

代码缓存在 `~/.product-knowledge/codebase/<org>/<repo>`；云端沙箱（环境变量 `OPENCLI_SANDBOX_MODE` 取 `1`）下代码由外部预置在 `~/workspace/`、扁平按 repo 名，改用那里。增量基准在 `<产品名>/.knowledge-state.json`，成篇蒸馏后应 `mark`（skill 一般会自动做）。

**地图与 json**：动过 `product-overview.md` 全景表，同轮必须改 `capability-map.json`。细则见 `references/capability-map.md`。

**写法细则**：细化读 `references/refine.md`；增量读 `references/incremental.md`。

**拷到别的知识库仓库**：整目录 `bootstrap-product-knowledge/`（含 `tools/`）拷过去即可。