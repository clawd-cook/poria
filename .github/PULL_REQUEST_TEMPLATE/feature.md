## 功能：[名称]

### 动机

<!-- 解决什么问题？对应 Issue：Fixes # -->

### 做法

<!-- 关键决策：类型放哪一层、IPC 字段名、是否新 Skill / 门禁 -->

### 用户可见变化

<!-- 桌面窗口里谁会碰到这条路径？ -->

### 契约

<!-- 无跨层字段可写「无」。有则列出： -->

| 字段 | 方向                                | 说明 |
| ---- | ----------------------------------- | ---- |
|      | 例如 TS camelCase → Rust snake_case |      |

- [ ] 后端仓仍不进入 `pipeline.repos`（若涉及启动流水线）
- [ ] `submit_pipeline` 仍发送 `backendTrdUrl`（若涉及启动流水线）

### 验证

- [ ] `pnpm typecheck`（改了 `src/`）
- [ ] `cargo test -p <crate>`（改了 Rust）
- [ ] 已在 `poria-desktop` 窗口走通主路径（若涉及 IPC）
- [ ] 未改 `submodules/`，未提交密钥与 `~/.poria/`

### 回退

<!-- 出问题如何撤。纯文档可写「还原提交」。 -->
