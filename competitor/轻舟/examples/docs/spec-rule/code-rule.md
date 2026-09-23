
## 规约

- **模块分层依赖方向**（单向，不得反向 / 越层）：
  - `pages → components → hooks/utils` 单向依赖，下层不得 import 上层。
  - pages 层禁止被 components 导入（页面组件不作为公共组件复用）。
  - services（API 层）不得依赖 components / pages（接口层不反向依赖视图层）。
  - stores 不得直接导入 components / pages（状态层不反向依赖视图层；组件通过 hooks 消费 store）。
  - components/common（基础组件）不得依赖 components/biz（业务组件）（通用组件不反向依赖业务组件）。

- **TypeScript 严格模式**：tsconfig 的 `strict: true` 不得关闭；禁止使用 `any` 类型（确实无法推断时用 `unknown` + 类型守卫）。
- **接口调用统一收口**：所有 HTTP 请求必须经过 `services/` 层的封装方法，禁止在组件内直接调用 axios/fetch。

## 经验

> 阶段性启发（这阶段怎么做更好），还没到强制；信心驱动可升级为规约。只收规定性启发，不收领域事实（那进 module-/tech-knowledge）。

## 踩坑记录
