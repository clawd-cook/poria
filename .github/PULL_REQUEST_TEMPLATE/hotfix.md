## 热修：[简述]

### 影响面

<!-- 哪个正式版本 / tag 坏了？是 DMG 还是 debug 窗口？ -->

- 版本 / tag：
- 运行环境：正式 DMG（`/Applications/Poria.app`） / `pnpm tauri dev`
- 出问题的位置：登录 / 需求 / 克隆 / 流水线 / 发布 / 其他

Fixes #

### 根因

<!-- 为什么正式路径会坏？不要贴 Cookie 或利用细节。 -->

### 修复

<!-- 最小改动是什么，为什么够用 -->

### 回归风险

- 级别：低 / 中 / 高
- 可能波及：

### 发布

- [ ] 合并后需要新 tag（`vX.Y.Z` 或 `vX.Y.Z-beta.N`）；旧 tag Re-run 不会带上这次修复
- [ ] 不需要发版（仅文档 / CI 说明）

### 验证

#### 热修前

1. 注明用的是正式 DMG 还是 `pnpm tauri dev`
2.
3.

#### 热修后

1.
2.

- [ ] 已在 **Poria 桌面窗口**确认（不是浏览器 `:1420`）
- [ ] 改了 Rust：已跑 `cargo test -p <crate>` 或 `cargo test --workspace`
- [ ] 改了 `src/`：已跑 `pnpm typecheck`
- [ ] 未提交 `~/.poria/`、密钥、仅本地变脏的 submodule 指针
