# workflow-run-autotest：跑 E2E 回归

进入后先判项目载体：微信小程序则转交 workflow-wx-autotest（plan.yaml + wx-auto-exec），不再走本 skill 的 Playwright/preflight 流程；其余项目跑 .workflow/scripts/test/tasks/<task>/ E2E、解析 junit、失败登记 issue。--no-autotest 跳过。

## 载体分流（进入后第一步，必做）

在物料闸 / preflight / Playwright **之前**判定项目是否为微信小程序。命中则**整段转交**，本文后续章节一律不执行。

**判定（命中任一即为小程序）**：

- 工程：根目录有 `project.config.json` / `project.private.config.json`，或 `package.json` 含 `@tarojs/*` / `taro build`+`weapp`
- 文档：本任务 `test-plan.md` / `materials.md` / `context.md` 写明微信小程序

**小程序 → 转交 `workflow-wx-autotest`（硬切）**：

1. Read `.claude/skills/workflow-wx-autotest/SKILL.md`，严格按该 skill 执行（有 plan.yaml 可直接段二跑测；没有则段一翻译再段二）。
2. **禁止**继续执行本文件下方任何章节：物料闸、preflight.sh、Playwright、easytrack 后置、本文件出口 checklist 的 H5 语义。
3. 出口仍属本节点：该 skill 跑完后，按它的结果决定——全绿则 `advance <task> --step workflow-run-autotest`；有失败则 `issue-add --from workflow-run-autotest` 并提示 `/workflow-feedback-loop`（与验证型节点纪律一致，不就地改代码）。
4. 转交后本 skill 即结束，不要「转交完又跑一遍 H5」。

**非小程序** → 忽略本节，按下方 H5 / Playwright 流程继续。前置补充：测试文件与 `playwright.config.ts`（或对应框架配置）就位。

## 物料闸（跑测前置）

物料**决策**已在 test-plan/test-cases 完成，这里只**执行**：照 `materials.md` 台账逐条备齐（启动 mock server / 注入登录态 storageState / 设置 feature flag），不重判档。

**维度标签 → 物料闸分支映射**：

| test-plan 维度标签 | 物料闸分支 | 含义 |
|---|---|---|
| `api` + inline mock | `[内联策略]` | local-route，无外部依赖 |
| `api` + local-mock | `[task-api-local-mock]` | whistle 代理本地 mock 文件 |
| `api` + easymock | `[task-api-easymock-*]` | 远程 easymock 平台 |
| `tracking` + local | `[内联策略]` | tracking-local，无外部依赖 |
| `tracking` + easytrack | `[task-tracking-easytrack]` | easytrack 平台验证（仅当 test-plan 包含 tracking 维度时触发） |
| `ui` / `logic` | `[通用]` | 仅需 storageState 注入 |

**按维度分诊检查**（分支名对应 test-plan `plan-N` 头部的维度标签 + test-cases 段 2 的策略决策；不认识分支时回 test-plan.md 反查该 plan-N 的维度和策略）：

```
物料闸：
├── [内联策略]              local-route / tracking-local → 无外部依赖，跳过
├── [task-api-local-mock]   ① 确认 mock 文件就位（src/mock.js 等）
│                           ② 启动 whistle 进程
│                           ③ 加载规则：`w2 add .workflow/scripts/test/tasks/<task>/whistle-rules.txt`
│                           ④ 配置 Playwright 浏览器 proxy 指向 whistle
│                           ⑤ 冒烟请求一条被 mock 的 URL 确认链路通
├── [task-api-easymock-*]   ① lbcli easymock <kind> --action template-get 探活模板存在
│                           ② 冒烟请求 easymock 地址确认测试环境路由通路
│                           ③ 通路不通 → 检查 test_env 配置，不可恢复时阻塞出口报人
├── [task-tracking-easytrack]（条件：仅当 test-plan 包含 tracking 维度时）
│                           triggers 文件存在 + SPM ID 有效（lbcli easytrack fetch 不报错）
└── [通用]                 storageState 注入（现有逻辑不变）
```

维度全为 logic 时只跑 [通用] 分支。

- **红线**：涉及真实第三方服务调用的 case，必须确认走的是沙箱/测试环境（= enter 上下文包中的 `env.target`）；环境缺省/存疑不跑。
- **残留摊牌**：只对 test-cases 标「⏳ 待确认」的新物料补摊牌，有就列给用户确认后再跑。正常路径零摊牌。

## 执行：只跑不修

**推荐入口**（preflight 脚本自动完成环境检查 + 跑测）：

```bash
bash .workflow/scripts/test/preflight.sh test --task <task>
# 或 local 模式（走 whistle 代理到本地 dev server）：
bash .workflow/scripts/test/preflight.sh local --task <task>
```

> 手动执行方式及参数细节见 `.workflow/scripts/test/README.md`「快速开始」和「手动执行」段。

**后置子阶段**（按 test-plan 维度 + 策略决策条件触发）：

```bash
# 条件：仅当 tracking 维度选了 easytrack-verify 或双开策略时触发
lbcli easytrack verify-triggers \
    .workflow/scripts/test/tasks/<task>/easytrack-triggers.ts \
    --env test \
    --output delivery/<task>/test/easytrack-report.json
```

产物：
- `delivery/<task>/test/junit.xml`——AI 解析用
- `delivery/<task>/test/test-results/`——trace / 截图 / HTML 报告，给人看
- `delivery/<task>/test/easytrack-report.json`——有 easytrack-verify 时（条件产物）

**AI 只读结构化产物**（`junit.xml` / `easytrack-report.json`），不把 trace/HTML 读进上下文。不重跑失败用例（flaky 由人判）、不修代码或用例（修复属 implement/test-cases）。

## Skip 用例根因归属（不准无条件放行）

完整协议见同目录 `skip-attribution.md`。核心红线：

- **禁止**未做根因归属直接保留 skip
- **禁止**把 CODE BUG 标为 skip（skip ≠ 「代码没实现但先不管」）
- 每条 skip 必须归入 CODE BUG / TEST ISSUE / VALID SKIP / ENV CONSTRAINT 四类之一
- CODE BUG / TEST ISSUE 必须建 issue 后才能 advance

## 失败先判真伪（源头过滤）

失败用例重跑一次；重跑通过 = 偶发环境噪音，不建 issue，报告记"重跑通过"。反复出现的同类环境失败按 TEST ISSUE 正常登记。复现的真失败才进「有失败」分支。判真伪不等于修——仍只跑不修。

## 出口检查清单

**AI 自动判，任一不过 → 不 advance：**

- [ ] 无 setup error / 无超时崩溃 / 无成片 skip（这些不算绿）
- [ ] 实跑用例数 ≈ test-cases.md 登记数（防文件被收集器漏掉）
- [ ] 所有 skip 用例已完成根因归属（见同目录 `skip-attribution.md`）
- [ ] 不存在未建 issue 的 CODE BUG / TEST ISSUE 类 skip
- [ ] （条件）有 tracking 维度时：`easytrack-report.json` 摘要已追加

**人侧（看 trace，AI 不读）**：抽查负向 case 截图——被测页面真显示了错误态，而非选择器失配的空过；baseURL 指向本期版本、登录态角色正确。人侧存疑→让人定夺。

**全通过**：junit 全绿 + skip 根因归属全为 VALID SKIP / ENV CONSTRAINT + easytrack 无 fail（如有）→ `advance <task> --step workflow-run-autotest`，打印报告路径传给 handoff-qa。

**有失败 / 有 bug 类 skip**：**严禁就地改代码/用例**（验证型节点无口头修复许可，见 PROTOCOL「反馈回路」红线）——对每条失败及每条 CODE BUG / TEST ISSUE 类 skip，`issue-add --from workflow-run-autotest`（现象写「用例名 + failure 节选 / skip 根因」，easytrack fail 写「SPM + 平台端判定」），`current_step` 不动，提示人工 review 后跑 `/workflow-feedback-loop` 分诊回退。
