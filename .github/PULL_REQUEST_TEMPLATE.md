## 摘要

<!-- 一两句说明为什么改，而不是列文件名 -->

## 关联 Issue

<!-- 有对应 Issue 时填写。安全问题不要在公开 PR 里贴利用细节。 -->

Fixes #

## 类型

- [ ] 缺陷修复
- [ ] 新功能
- [ ] 破坏性变更
- [ ] 文档
- [ ] 重构（行为不变）
- [ ] CI / 发布
- [ ] 测试

## 改动范围

<!-- 勾选实际动到的层。不要反转 crate 依赖；不要改 submodules/ 里的文件。 -->

- [ ] `src/` 前端
- [ ] `src-tauri/` IPC
- [ ] `poria-core`
- [ ] `poria-commands`
- [ ] `poria-skills`
- [ ] `poria-channels`
- [ ] `poria-resources`
- [ ] `poria-infrastructure`
- [ ] `.github/` / 发布

跨层字段（需求筛选、`backendTrdUrl`、前端 `TRD.md` / 后端 `BACKEND_TRD.md`）如有变动，写明前后契约：


## 验证

- [ ] `src/` 有改动：已跑 `pnpm typecheck`
- [ ] `crates/` 或 `src-tauri/` 有改动：已跑 `cargo test -p <crate>` 或 `cargo test --workspace`
- [ ] 涉及 `invoke` / 登录 / 需求 / 克隆 / `submit_pipeline`：已在 **Poria 桌面窗口**验证，不是浏览器 `:1420`
- [ ] 涉及 Deploy / EasyCI：确认 **先 push 再 SELECT 绑定**
- [ ] 前端格式：`pnpm exec oxfmt .`（若改了 `src/`）

### 复现 / 验证步骤

1.
2.
3.

## 检查

- [ ] 未提交 `~/.poria/`、`workspace/db/`、Cookie、证书、私钥
- [ ] 未提交仅因本地 checkout 变脏的 submodule 指针
- [ ] 未在生产路径打开 `PORIA_PIPELINE_FIXTURE=1`
- [ ] 日志与截图已打码敏感信息
