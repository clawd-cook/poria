# 项目概览

> AI 上下文压缩包，每次会话都加载。本文为示范填充；新项目接入请整体替换，或调 `/project-overview-bootstrap` 生成。
> 标 `_[AI 推断/待 verify]_` 的部分由 generator 推断，请优先 review。

## 一、项目身份

- **一句话定位**：xxx管理后台——为运营人员提供xxx的创建、编辑、审核能力。

- **领域术语**（看到这些词默认指业务概念，不要按通用名词理解）：

| 术语 | 等价/别名 | 一句解释 |
|------|----------|----------|
| xxx | xxx | xxx |
| xxx | xxx | xxx |

## 二、项目地图

> 写「在哪 + 怎么识别 + 关键示例」，不做全量枚举。技术栈混乱时直接讲混乱、不强求统一。

**目录约定**
- src/pages/：页面组件，按业务域分目录
- src/components/：公共业务组件
- src/services/：API 调用层
- src/stores/：状态管理

**路由**
- 配置式路由 / 文件系统路由，核心页面：/xxx、/yyy、/zzz

**组件体系**
- UI 框架：Ant Design / Element Plus / 其他
- 业务组件：src/components/biz/

**状态管理**
- 方案：Redux / Zustand / Pinia / 其他
- store 目录：src/stores/

**接口调用层**
- 封装：axios / fetch / umi-request
- 目录：src/services/
- 鉴权：cookie / token（存储位置：localStorage / cookie）

**样式方案**
- CSS Modules / Tailwind / Less / styled-components
- 主题 token：src/styles/theme.ts

**构建与工程化**
- 构建：Vite / Webpack / Next.js 内置
- 包管理：pnpm / npm / yarn
- 环境配置：.env.development / .env.production

**框架栈**：React / Vue + xxx（详见 `docs/tech-knowledge/` 相关文档）

## 三、项目特有红线 + 知识索引

**红线**（通用规则在 `docs/spec-rule/code-rule.md`，本节只列本项目特有的）：

- xxx（示例：所有异步请求必须经过统一拦截器，禁止裸调 fetch/axios）

**触发式索引**（只列「关键词不易直接桥接」的横切技术；业务模块文档在 `module-knowledge/` 下，按业务名直接查找）：

- 涉及xxx → `tech-knowledge/xxx.md`
