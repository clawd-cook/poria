# Xingyun related-to-me demand list

Parent: `09-17-delivery-workspace` (R3). Technical shape: parent `design.md` and `research/h2o-demands-and-repo-path.md`.

## Goal

用 SSO 凭据拉取「与我相关」的行云需求，在需求 Tab 展示搜索、分页、「由我受理」筛选和行内「开始」。

## Requirements

- Xingyun `listDemands` 默认不传 `receiver`：JACP 按 cookie / `optErp` 返回与当前 ERP 相关的进行中需求
- 复选框「由我受理」勾选后才传 `receiver = 当前 ERP`（原 assigned-to-me）
- 关键字搜索 + 分页；不要「我提出的」或项目全量
- 行字段：名称、编号、状态、接收人；右侧「开始」
- 列表接口通常不返回 `receiver` 对象：未勾选时不要把当前 ERP 填进接收人；勾选时才可用查询 receiver 作为回退
- 未登录 / cookie 失效 / 失败：引导登录或重试，不造数据

Must wait: `desktop-ia` 需求 Tab；`09-17-sso-login` 的 cookie 可用。

## Out of Scope

- 向导、resolvePrdLink 预填、创建 Pipeline（`start-pipeline`）
- communicate / accept

## Acceptance Criteria

- [ ] 已登录默认列出与当前 ERP 相关的需求；勾选「由我受理」后只列 receiver=当前 ERP
- [ ] 搜索与分页可用；切换复选框重置到第 1 页
- [ ] 未登录不请求接口并提示登录
- [ ] `cargo check --workspace` 与 `pnpm typecheck` 通过
