---
name: xingyun-release
description: 行云自动化发布。把前端/后端应用发布到预发布/测试环境的指定分组：默认走完整 release 流程（构建 → 部署 → 观测服务启动状态），构建/部署已被其他流程触发时自动转为等待观测；缺目标分支构建任务时自动拷贝最新成功执行记录创建任务；也支持强制重新构建、仅新建构建任务、选择已有制品对分组/机器部署。基于 lbcli 的 xingyun 原子能力组装，省 token。仅当用户要执行一次发布/部署到分组时使用；只想看状态用 xingyun status。触发关键词：行云发布、jdos 发布、部署到分组、发布到预发、发布到测试、强制重构建、选制品部署、新建构建。
---

# 行云发布 (xingyun-release)

把当前仓库的代码发布到行云某个**预发布/测试**分组。本 skill 是 `lbcli` 的 `xingyun` 原子能力的**轻量编排**——绝大多数发布只需一条 `lbcli xingyun release`，特殊诉求才拆成原子命令。

不走浏览器 DOM 操作。**所有 lbcli 命令的窗口/profile/SSO/JSON 引号等工具纪律，一律遵循 `lbcli` skill，本文不重复。**

## 前置约定

- 当前工作目录 = 正在发布的仓库；当前 `HEAD` = 要发布的 commit。
- **前/后端、镜像 vs OSS 包，CLI 自动探测**（`deployType=oss`→前端 / `jdos_container`→后端）。用户**无需**传 app_type。
- **生产分组由 CLI 硬拒绝**，本 skill 不重复护栏，也不绕过。
- **分组可以不指定**：`--group` 和 `--group-intent` 均可省略，默认等同于 `--group-intent pre`（自动从接口返回的分组列表中找 `environment=pre` 的唯一分组）。多个命中时 CLI 报错并列出候选，此时需要用户明确指定。
- 用户只给 `app` 即可，`branch`/`commit` 缺省时 CLI 自动读 `git branch --show-current` / `git rev-parse --short HEAD`。在 workflow-deploy 中为了审计，优先显式传 branch/commit。

## 输入

| 字段 | 必填 | 说明 |
|---|---|---|
| `app` | 是 | 行云应用名，如 `wecom-scrm-platform`、`scrm` |
| `group` | 否 | 部署分组名（精确），与 `group-intent` 二选一；均不填默认 `pre` |
| `group-intent` | 否 | 分组意图（见下表），`group` 已填时忽略 |
| `env-type` | 否 | 默认 `pre`。`pre`=临时预发 / `pubpre`=固定预发 / `test`=测试环境。**后端测试环境应用（如 `eone-` 前缀）必须显式传 `--env-type test`，否则所有命令（`groups`、`release`、`deploy` 等）均会走预发 tenant 导致「应用不存在」。** |
| `branch` | 否 | 缺省读当前 git 分支 |
| `commit` | 否 | 缺省读当前 git 短 HEAD |

### group-intent 取值

| 用户说法 | 传给 CLI 的值 |
|---|---|
| 发布到预发布 / 预发 / 不指定 | `pre`（默认） |
| 发布到固定预发 / 稳定预发 | `prepub` |
| 发布到临时预发 | `pre`（environment=pre 且 subEnv=pre） |
| 发布到测试 | `test` |
| 发布到 stage 分组 / 发布到某昵称 | 传昵称或分组名关键字，如 `stage`、`固定预发环境` |

## 决策：先判断用户要哪种发布

按用户意图选**一条**路径，不要叠加：

### 1. 默认 —— 完整发布（构建 → 部署 → 观测）

最常见。用户说"发布到 X 分组""部署一下""上线到预发"且没有特殊限定时走这条。

```bash
lbcli xingyun release --app <app> --group <group> [--env-type pre] -f json
# 测试环境后端应用必须加 --env-type test，同时 --branch 和 --commit 需显式传目标分支/commit
# （因为当前 git HEAD 可能是其他分支，CLI 默认读当前 HEAD 会导致 branch/commit 不匹配）
lbcli xingyun release --app <app> --group <group> --env-type test --branch <branch> --commit <commit> -f json
```

`release` 已内置全部编排，**无需手工串原子命令**：
- 缺目标 commit 制品 → 自动触发构建并等待；
- **缺目标分支构建任务 → 自动拷贝最新成功执行记录（按 `build_time`）的完整模板创建任务**（含系统基础镜像），再触发构建；**无需先跑 `create-build`**；
- **构建/部署已被别人或平台触发（进行中）→ 自动检测并等待收敛**，不重复触发；
- 分组已在目标制品 → 跳过部署；
- 后端部署后自动做健康检查（容器运行中/健康检查成功）。

这条天然覆盖了"**被其他流程触发时进入等待观测**"的诉求——默认就是 ensure，进行中会等。

### 2. 只观测、不要触发任何写操作

用户明说"只看不要动""别触发构建/部署，等它自己好"时，加 observe-only：

```bash
lbcli xingyun release --app <app> --group <group> \
  --build-action observe-only --deploy-action observe-only -f json
```

observe-only 下若目标 commit 没有制品 / 分组未在目标制品，直接返回 `FAILED`（不触发）。

> 用户只是"看一眼当前状态"而非"发布"时，**不要用 release**，用更轻的：
> `lbcli xingyun status --app <app> --group <group> -f json`

### 3. 强制重新构建

用户说"强制重构建"、"重新打个包"、"不要复用旧制品"、"重新构建" 时，加 `--force-build`（跳过制品复用与平台自动构建等待，直接触发新构建后继续部署+观测）：

```bash
lbcli xingyun release --app <app> --group <group> --force-build -f json
```

### 4. 选择已有制品，对分组/机器部署（不重新构建）

用户说"用某个已有制品部署""回滚到上个版本""指定机器部署"时，**先列制品供选，再部署**：

**Step A —— 列出最近制品供用户选**（用户已明确给出 image 全名或 commit 时跳过本步）：

```bash
lbcli xingyun artifacts --app <app> [--commit <prefix>] [--limit 20] -f json
```

把 `image / branch / commit / created` 列给用户，让用户挑一个。

**Step B —— 用选定制品部署到分组**：

```bash
# 按 image 全名（最精确），group 可省略（自动推断预发分组）
lbcli xingyun deploy --app <app> --image <image> -f json
# 指定分组
lbcli xingyun deploy --app <app> --group <group> --image <image> -f json
# 或按 commit 匹配最新制品；用 group-intent 表达意图
lbcli xingyun deploy --app <app> --group-intent prepub --commit <commit> -f json
```

可选后端参数：
- `--ips ip1,ip2`：只部署分组内指定机器（默认全部实例）；
- `--mode ip|cluster`：`ip`=指定IP grayUpdateCluster（默认）/ `cluster`=updateCluster。

`deploy` 默认 `--wait true`，会等部署收敛；分组已在目标制品会返回 `skipped`。

> `deploy` 本身**不做后端健康检查**（那是 `release` 的 health 阶段）。如果用选制品路径但仍想要健康观测，部署后补一条：
> `lbcli xingyun status --app <app> --group <group> -f json` 或直接改用 `release --deploy-action ensure`（它会等部署 + 健康）。

### 5. 仅新建构建任务（不部署）

用户只要「拷贝最新构建记录 → 改分支 → 保存并执行」，**不要部署到分组**时，用独立命令（`release` / `build` 在缺任务时也会自动走同一套拷贝逻辑）：

```bash
# 拷贝最新成功执行记录的配置，创建目标分支任务并触发构建
lbcli xingyun create-build --app <app> --branch <branch> --wait -f json
# 仅保存任务、不触发构建
lbcli xingyun create-build --app <app> --branch <branch> --save-only -f json
# 指定拷贝源任务 ID（默认取最新成功执行记录）
lbcli xingyun create-build --app <app> --branch <branch> --source-task-id <taskId> -f json
```

## 结果解读（release 的 JSON 字段）

| 字段 | 含义 |
|---|---|
| `result` | `SUCCESS` / `FAILED` / `TIMEOUT`，唯一可信的闸门结论 |
| `deployKind` | `backend` / `frontend`（CLI 自动探测结果） |
| `artifactVersion` | 实际部署的制品/镜像版本 |
| `buildStatus` / `deployStatus` / `healthStatus` | 各阶段状态 |
| `failureStage` | 失败阶段：`build_trigger`/`artifact`/`deploy_not_started`/`health`/`timeout`（异常兜底时为 `compile`/`deploy`），成功为 `none` |
| `failureReason` / `evidence` | 失败原因与证据（含 logbook 排查提示） |
| `timingJson` | 各阶段耗时 |

向用户汇报：成功 → 报 `artifactVersion` + 总耗时；失败 → 报 `failureStage` + `failureReason`。

## 失败时：自动拉日志初步定根因

`result != SUCCESS` 时，**不要只把错误丢回给用户**。按 `failureStage` 主动取证据，给出初步根因：

- `failureStage = health` 或 `deploy`（服务起不来/部署不收敛）：
  ```bash
  lbcli logbook grep --app <app> --keyword "ERROR" -f json
  ```
  （`evidence` 里通常已附上这条命令）从日志里找启动报错、端口冲突、依赖连不上等，给出"初步根因 = XX"。
- `failureStage = compile`（异常兜底的构建失败）：
  ```bash
  lbcli xingyun build-log --app <app> --build-id <buildId> --stage build -f json
  ```
  从构建日志定位编译/打包错误。若 `failureReason` 含 `no configured build tasks found to copy from`，说明应用下没有可拷贝的完整构建模板，需先在行云页面手工建一条成功构建记录。
- `failureStage = artifact`：制品 commit 与目标 commit 不符——通常是构建任务匹配了别的分支，提示用户核对 branch/commit。
- `failureStage = build_trigger` 或 `deploy_not_started`：**只在 observe-only 模式出现**，含义是"目标 commit 还没制品 / 分组还没在目标制品，而你要求了不触发"。这不是真故障——告诉用户"目标尚未就绪"，并询问是否改用默认（ensure）模式去触发构建/部署。
- `failureStage = timeout`：报告卡在哪个阶段、已耗时多久，建议用户确认平台是否拥堵或提高超时。

日志命令的窗口/引号纪律遵循 `lbcli` skill。若初步证据仍不足以定根因（链路复杂、跨服务），**引导用户转 `trouble-shooting` skill** 做深入排查，不在本 skill 里无限重试。

## 与其他工具的边界

- 只想看状态、不发布 → `xingyun status`（别起 release）。
- **不知道分组名** → 先用 `lbcli xingyun groups --app <app> [--env-type test] -f json` 列出所有预发/测试分组（已过滤生产），看 `groupName` + `nickname` 再决定传哪个 `--group`。测试环境应用必须加 `--env-type test`，否则返回空或「应用不存在」。
- 看分组机器/实例 → `lbcli xingyun pods --app <app> --group <group>`。
- 深入根因排查 → `trouble-shooting` skill。
- 发布完成后要触发行云**部署到分组以外**的流程（如行云页面侧的灰度切量编排）不在本 skill 范围。
