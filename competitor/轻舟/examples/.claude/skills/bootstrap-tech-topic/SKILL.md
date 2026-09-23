---
name: bootstrap-tech-topic
description: "挖掘项目值得沉淀的 tech-knowledge topic，回填 project-overview 触发式索引。只产索引行、不写文档。新项目接入时由 bootstrap 一键流程在 project-overview 落盘后串行调用；也可手动触发补挖。被挑中的 topic 由 bootstrap-tech-knowledge 逐个细化成文档。"
---

# bootstrap-tech-topic

挖掘阶段，不是生成阶段：通读项目，挖出值得沉淀的 tech topic，回填 `docs/project-overview.md` 的触发式索引。**只产索引行，不写文档正文**——被选中的 topic 由 `bootstrap-tech-knowledge <主题>` 逐个细化。

## 挖掘提示词

你现在需要向用户提供一些 tech-knowledge 的 topic 推荐。

请你通读项目，去挖掘出来一些你觉得整个项目有亮点的技术设计，平平无奇的通用知识不要。

这些技术设计通常都是一个顶层能力，会作用于项目很多地方。

而且因为比较复杂，AI 在需求中直接梳理容易漏掉或者犯错；或者虽然能推对，但每次现场推理都很费上下文，提前沉淀就能省掉这趟重复开销。

或者说这个内容背后的设计意图很重要，AI 直接读取容易理解偏。

那么这样的内容，我们提前挖出来、推荐给用户确认就很有必要。

为了防止遗漏，先尝试找全，然后在排序，最后找出 1-5 个这样的 topic，当然如果少了也没必要硬凑，有价值才推荐。

## 回填索引

挖到的 topic 直接回填到 `docs/project-overview.md` 的「触发式索引」区，参照 module 索引的写法：一行一条，每条「触发关键词 → `tech-knowledge/<主题>.md`」，标 `_[AI 推断/待 verify]_`。这一步只产索引行、不写文档正文。

## 留挖掘草稿（给下游细化窗口当交接便签）

挖掘和细化一定是两个窗口：挖掘窗读完上下文已满，细化时另开新窗，挖掘时的判断全丢。所以把挖掘结论留一份轻量草稿，写进 `docs/tech-knowledge/_topic-notes.md`，让细化窗口不必从零重读。

- **每条 topic 只记几行**：为什么入选（AI 会怎么栽 / 费什么上下文）、挖掘时撞到的关键线索。
- **是交接便签，不是成稿**：别在这里写原理、写实现——那是 `bootstrap-tech-knowledge` 细化时的活。多写既越界又多一份会腐烂的内容。
- **会过时**：`_` 前缀标明它是过程草稿；下游细化某 topic 定稿后会删掉对应段。

## 和其他 skill 的边界

| skill | 职责 |
|------|------|
| **`bootstrap-tech-topic`（本）** | **挖 topic + 回填 overview 触发式索引（不写文档）** |
| `bootstrap-tech-knowledge` | 下游：把挑中的 topic 细化成 `tech-knowledge/<主题>.md` |
| `bootstrap-project-overview` | 上游：先生成 overview 当地图；本 skill 在其落盘后跑 |
