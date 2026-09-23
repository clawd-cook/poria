---
name: bootstrap-tech-knowledge
description: "生成项目的 docs/tech-knowledge/<主题>.md——把项目特有的横切技术机制沉淀给 AI 加载。AI 准备处理某横切机制时按主题查找。"
---

## 前置校验
用户未指定话题，拒绝执行。（话题从哪来：1.由 `bootstrap-tech-topic` 挖掘并回填到 project-overview 触发式索引，用户从中挑选后调用本 skill 逐个细化。2. 也有可能是用户自己指定的新的topic）

## 先读挖掘草稿（如果有的话）
开写前先看 `docs/tech-knowledge/_topic-notes.md` 里本 topic 那段，可以参考一下topic的具体内容，但是不要全信，因为topic的内容挖掘是一个通用任务，对具体细节不够深入。
文档定稿后，删掉该 topic 在 `_topic-notes.md` 的对应段（草稿用完即清；该文件清空后可一并删除）。

## 内容生成提示词

1.为什么要有全局技术？一定是遇到了什么通用问题，所以需要先分析出来这个通用问题是什么？
2.分析出来之后，把全局技术的实现原理站在较高的视角把它讲清楚，不要陷入细节，如果是行业通用技术，原理需适当收敛篇幅。
3.之后把全局技术处理到什么程度、做了什么讲一下？没做什么不用讲。
4.最后一句话总结这个全局技术，放到开头。

预算不设上限，一定要把问题讲清楚。
文档生成完毕后，去project-overview.md维护一下索引。

---

## 和其他文档的边界

| 文档 | 职责 |
|------|------|
| `spec-rule/` | 团队级通用规范（适用所有项目） |
| **`tech-knowledge/`** | **本项目特有的通用技术设计** |
| `module-knowledge/` | 单业务模块的业务知识 |
| `project-overview.md` | 触发索引 + 极简红线，**指向** tech-knowledge 但不展开 |
| `bootstrap-tech-topic` | 上游：挖出值得沉淀的 topic 回填索引；本 skill 消费它挑出的 topic |
