# init：收集任务级配置（profile + 环境）

> 敲 `/workflow-engine init <task>` 时按需读本文；其余命令用不到，不必读。

**先定任务名**（kebab-case）：取需求相关语义的名字；

init 要落 2 类配置进 `state.json`：**profile**（编排模式，决定节点集/顺序/闸门；AI 编排下必须显式带 `--profile`）、**env**（运行环境 local/test/gamma 默认 test、测试范围 http/jsf 可多选默认 http，+ 范围派生项 HTTP 域名 / JSF 别名 / eone 特性环境）。**无端到端自动化测试的 profile（`has_e2etest:false`，如 lite / plan-mode / *-no-autotest）跳过全部环境收集、env 钉 `target=local`**——无自动化测试、无需触发部署，测试环境坐标无处消费；deploy 据 `target=local` 只 commit/push 不触发真部署。收集分两条路径：

**AI 编排路径（你接到 init 请求时走这条，主路径）**：别直接裸调 `init` 让它走默认值——先把配置问清楚，再拼 flags 一次调 `init`。步骤：
1. 先定 **profile**：复用 workflow-start 已选定的 profile，直接带上即可。只有上下文里确实找不到时，才 `AskUserQuestion` 重新问（列 `profiles` 里的模式与节点组合，默认 `default_profile`）。**选中 profile 的 `has_e2etest` 决定后续要不要问环境**：`false`（无自动化测试，如研发/精简/vibe）→ 跳过第 2-3 步全部环境提问，直接第 4 步只带 `--profile` 调 init（env 自动钉 local）；`true` → 继续问环境。
2. profile 含端到端自动化测试（`has_e2etest:true`，如 standard-openspec）时，问 **运行环境**、**测试范围**（http/jsf 多选，默认 http）。
3. 据答案追问自由文本条件项：范围含 http → **HTTP 域名**（必填）；含 jsf → **JSF 别名**（必填）；环境=test → **eone 特性环境**（非必填，可空）。
4. 拼成 flags 调一次 init：`--profile <名> --env <t> --scope <http[,jsf]> --http-domain <d> --jsf-alias <a> --eone <e>`（默认值项可省 flag；无 e2e 的 profile 只需 `--profile <名>`）。`--profile` 恒为必带。**注意 `--profile` 只定编排模式、不短路环境收集**——tty 下即使带了 `--profile`，向导仍会收集环境（除非该 profile `has_e2etest:false`）；只有显式给了 env flag（`--env`/`--scope`/…）才走非交互 flag 路径。**代码兜底（两道 loud fail，都不留孤儿目录）**：非 tty 下未带 `--profile`、或需环境的 profile 没给任何 env flag，init 都会直接报错逼你回头补齐——「别裸调」不只是约定，裸调会被挡下。
5. init 后**把需求起点写入 `delivery/<task>/context.md`**（plan/change/prd 位置或一句话描述）——供切窗口/新窗口/external-window 节点冷启动找回起点。CLI 不碰这个文件，由你写。

想改 env（环境/范围/域名/别名/eone）用 `set-env <task> --env|--scope|--http-domain|--jsf-alias|--eone`——**部分更新**（只动给了的字段，可空字段传空串清空），env **全程可改**；


