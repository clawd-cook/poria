# Xingyun assigned-to-me demand list

Parent: `09-17-delivery-workspace` (R3). Technical shape: parent `design.md` and `research/h2o-demands-and-repo-path.md`.

## Goal

用 SSO 凭据拉取「指派给我」的行云需求，在需求 Tab 展示搜索、分页和行内「开始」按钮（按钮可先禁用，由 `start-pipeline` 接向导）。

## Requirements

- Xingyun 增加 listDemands；receiver 固定为当前 ERP（assignedToMe）
- 关键字搜索 + 分页；不要「我提出的」或项目全量
- 行字段：名称、编号、状态、接收人；右侧「开始」
- 未登录 / cookie 失效 / 失败：引导登录或重试，不造数据

Must wait: `desktop-ia` 需求 Tab；`09-17-sso-login` 的 cookie 可用。

## Out of Scope

- 向导、resolvePrdLink 预填、创建 Pipeline（`start-pipeline`）
- communicate / accept

## Acceptance Criteria

- [ ] 已登录能列出指派给当前 ERP 的需求；搜索与分页可用
- [ ] 未登录不请求接口并提示登录
- [ ] `cargo check --workspace` 与 `pnpm typecheck` 通过
