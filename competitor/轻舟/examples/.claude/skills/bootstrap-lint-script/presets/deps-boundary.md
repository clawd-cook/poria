# preset: deps-boundary（分层依赖边界）

**意图**：锁住多模块分层架构的依赖方向——api 契约层不依赖实现层、service 不反向 import controller、repository 不反向调用 service、controller 不绕过 service 直连 repository。这类违规编译器不拦（Maven 依赖传递可见即合法），靠人盯必漏。

**适用性探测**：项目是多模块 Maven 工程且按 api/controller/service 角色拆模块（看根 pom 的 `<modules>` + 根目录 ls）。单模块项目或分层只体现在包结构 → 规则意图仍适用，但 `applies` 要从「模块名判断」改写成「包前缀判断」，等于半重写，按现场生成处理而非填参。

**落地方式**：拷贝 `deps-boundary.py` → `.workflow/scripts/lint/lint-deps.py`，填顶部 CONFIG 区五个参数。模板自带双闸：FILL-ME 未填拒跑、扫 0 文件拒过。

**探参**：

- `BASE_PACKAGE`：任一模块 `src/main/java` 下最深公共包路径，或由根 pom groupId 推断后向人确认。
- `MODULE_PREFIX` / `API_MODULE` / `CONTROLLER_MODULE` / `SERVICE_MODULE`：根目录 ls 与根 pom `<modules>` 对照。
- 模块角色对不上号（如没有独立 api 模块）→ 删掉对应 Rule，别硬填。

**落地后核对 RULES**：规则里的包结构假设（`service.repository` / `service.service` / `service.model.domain` / `api.dto`）按项目实际分层增删改。全量跑一遍：误报为零或逐条确认为真违规；存量违规多的规则可先降 warning 暴露技术债，别让闸门一上来就堵死主干。
