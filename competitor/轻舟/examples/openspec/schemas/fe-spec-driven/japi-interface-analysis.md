# JAPI 接口分析流程

> fe-spec-driven schema 专用。按 `japi-interface-analysis.md` 触发时加载本文件。
>

## 隔离策略

| 条件 | 处理方式 |
|---|---|
| 接口数 ≥ 2，或单接口出参字段 > 20 | spawn subagent 隔离执行 |
| 单接口且字段 ≤ 20 | 主 agent 直接跑 `lbcli japi` 命令 |

## 第一步：拿到 appCode（及入口接口详情）

**若用户给的是 japi 方法链接（带 `methodId`，形如 `.../demandManage/34639?methodId=5064971&branchId=53132`）**：直接 `lbcli japi get-by-url "<url>"` 一步拿到**入口接口**的 appCode + path + method + 完整 input/output。（URL 里 `demandManage/{N}` 的 N 是 appId 数字、不是 appCode）拿到 appCode 后进第二步，对待解析清单里的接口逐个取定义。

**否则**，`lbcli japi` 按 **JDOS 应用英文名**（`appCode`）查询。按下面顺序找，拿到即停：

1. **项目根目录的 `lbcli.config.md`**：`## J-API` 段下的 `appCode`（值为 `需补充` 或缺失时视为未配置，走第 2 步）
2. **没有 → 主动向用户索取**：「请提供 JDOS 应用英文名（后端应用标识，例：wx-precision-delivery）」。拿到后建议写回 `lbcli.config.md`，下次免问。

## 第二步：取接口定义

对**待解析清单**里的每个接口取定义（清单在 explore 阶段已按需求圈定，见 `context.md` 接口列表；本步不重新判断哪些接口相关）：

1. **入口接口**（若来自 japi 链接）：详情已由 get-by-url 拿到，直接用
2. **其余接口**：`lbcli japi get-method <appCode> --http-path <path> --http-method <method> -f json` 逐个取完整 `input` / `output` 树
3. 清单只给了 appCode、未定具体接口时：`lbcli japi list-methods <appCode> --method-type http -f json` 列出，把 path / method / 简介列给主 agent / 用户确认后再逐个取定义

## 第三步：提取契约

1. 提取前端消费侧关键信息，去重嵌套 VO
2. 归纳接口间数据传递关联
3. 返回精简契约表

### 信息提取原则

- **入参**：只列业务参数，跳过通用鉴权参数
- **出参**：展开到前端实际消费的字段为止，跳过前端不使用的中间 VO 包装层
- **枚举/状态值**：完整保留（前端分支判断核心）
- **VO 去重**：同一 VO 多接口出现时只首次展开，后续标注引用

**就绪判断**：

| 状态 | 判据 | 处理 |
|------|------|------|
| 已就绪 | `list-methods` 有条目，`get-method` 返回完整 `input`/`output` | 正常生成 service |
| 未就绪 | `list-methods` 未列出，或 `get-method` 抛 `未查到方法详情` | service 函数体写 `// TODO: 待后端接口就绪后对接`，**不要凭命名猜后端结构**，避免返工 |

## 返回要求

每接口包含：路径、关键入参、关键出参（字段路径+类型+业务含义）、枚举状态值、前端判断逻辑。多接口时补充接口间数据传递关联。

## 落盘

写入 `delivery/<task>/workspace/api-contracts.md`。

## 端到端示例

> explore 阶段已把待解析清单定为 `checkPin`（入口，来自 japi 链接）+ `bindPin`（同登录流上下游），本流程负责取定义 + 裁剪。

1. 入口接口 `lbcli japi get-by-url "https://j-api.jd.com/fe-app-view/demandManage/34639?methodId=5064971&branchId=53132" -f json`
   → 一步拿到 `appCode=interactive-delivery-soa`、`GET /wx/miniGameLogin/checkPin` + 完整 input/output
2. 其余接口 `lbcli japi get-method interactive-delivery-soa --http-path /wx/miniGameLogin/bindPin --http-method POST -f json`
3. 对入口 + 关联接口按信息提取原则裁剪，归纳接口间数据传递，落契约表

## 禁止项

- ❌ 将完整 response schema 原样返回主 agent
- ❌ 忽略枚举值/状态码
- ❌ 有 j-api.jd.com 链接却不跑 `lbcli japi` 命令，仅凭链接文字猜测接口内容
