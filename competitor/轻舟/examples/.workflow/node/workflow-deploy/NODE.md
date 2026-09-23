# workflow-deploy：提交并部署

提交代码到远端、触发部署。**commit/push 不可逆且对外**，必须人工确认 commit 内容后才推。前置：working tree 无非本任务脏改动、分支符合项目约定。
默认使用`xingyun-release` skill 进行镜像的构建和部署。

## 起手：无可部署变更检测（幂等跳过）

deploy 是幂等的——同一份产物部署两次是 no-op。本节点可能被**重复路过**（run-autotest 回退 test-cases 只改了测试代码、或本次需求纯文档/配置无代码变更），不该每次都硬停等人。所以**起手先判有没有可部署变更**：

1. 取基线：`deploy` 角色区域记录的上次部署 commit hash；首次部署则取本任务起点，如 `master`）。
2. 算 diff：`git diff --name-only <基线>..HEAD`，**过滤掉非部署路径**——`delivery/`、`docs/`、`.workflow/`、`trd` 角色区域（如 `openspec/`）、`schema-draft` 角色区域等草稿（这些不影响线上运行物）。
3. 判定：
   - **过滤后为空** → 本趟无可部署变更，**no-op 跳过**：在 `deploy` 角色区域记一行（`无可部署变更，跳过 — 对比基线 <hash>`），直接 `advance`，**不硬停、不问人**（run-loop 自驱与人工模式都跳）。
   - **过滤后非空** → 有真实部署物，照常走下面的上线前置闸门 + 硬停确认。

> 这一步把「没东西可部署却被拦」的差体验消掉；同时硬停只在真有变更时触发，钉死语义不变。

## 上线前置闸门（commit/push 前强制）——为什么单设

中间件「代码写好但线上没配 / 没建 / 没权限」会**逃逸到生产运行时**——它穿透 lint/CR/test 所有闸门，线上一跑才炸。所以 push 前必须从 diff 反推上线前置项、当面逐条确认，**不依赖任何人记得在编码时登记**。

1. **反推触点**：`git diff master...HEAD`，按 `docs/tech-knowledge/DongBoot-and-Middleware-Overview.md`「中间件上线前置」表的 **diff 识别签名**列，扫出动了哪些中间件。
2. **拉前置列给用户**：对命中的中间件取「配置类前置」+「线上人工操作」两列，分两组列在对话里。
3. **硬停确认**：逐条念，重点是「线上人工操作」。DDL 类额外确认**执行顺序**（建表/加字段须早于依赖它的代码上线）。**人未逐条确认完成，不得 commit/push/触发部署。**
4. 确认后在 `deploy` 角色区域写**一行留痕**（`上线前置: 已确认 — <列了啥>`，无触点写「本次无中间件上线前置」）。
5. **是否触发真部署看 `env.target`**（取 `state.json` 的 env 块）：`target == local` 时**代码照常 commit/push，但不触发部署、不等部署**——用于无自动化测试诉求的流程（没有自动化回归来验证部署结果，就别自动按下部署按钮，推上去后由人/QA 决定怎么部署）。`target != local` 时照常触发部署 + 等部署完成。**注意 local 不再是"不提交"——commit/push 任何 target 都做，local 只关"要不要触发部署"这一个开关。**

> 表是经验基线：diff 没命中签名的不列；具体配置项以项目实际为准。

## 提交（按约定 deploy 一定成功，只做最薄提交+登记）

> **`env.target` 是环境语义（local/test/gamma），不是行云分组名**
汇总 `git status` + `git diff --stat` → 生成 commit message 草稿（`<feat|fix|refactor|docs>: 摘要` + 关联 PRD/TRD/影响模块）→ **人工确认后** `git add <具体文件>` + commit + push →（`target != local` 才）触发部署（项目实际命令）→ 写 `deploy` 角色区域（上线前置留痕 + commit hash + branch + 时间 + 触发方式 + 链接 + **部署目标环境 + baseurl**；`target == local` 时触发方式记「仅提交未触发部署」、部署目标环境记 local）。

只 add 具体文件、别 `git add -A`/`.`——误把 `.env`/workspace 草稿卷进 git 历史就拔不掉了（不可逆）。上线前置未逐条确认 → 不 push。

## 出口

`advance <task> --step workflow-deploy`，照 CLI 打印的闸门指令当场接续。
