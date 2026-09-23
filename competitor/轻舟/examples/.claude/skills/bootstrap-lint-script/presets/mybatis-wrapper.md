# preset: mybatis-wrapper（MyBatis-Plus Wrapper 字符串列名）

**意图**：禁用 MyBatis-Plus 非 Lambda Wrapper、字符串字面量列名和 `setSql(...)`——字符串列名绕过类型系统，字段重命名时静默失效，逻辑删除/多租户等横切条件也无法被工具审计。

**适用性探测**：`grep -rl "com.baomidou" --include=pom.xml .` 命中即适用；不用 MyBatis-Plus 的项目跳过本 preset。

**落地方式**：原样拷贝 `mybatis-wrapper.py` → `.workflow/scripts/lint/lint-mybatis-wrapper.py`，零填参——规则只认 MyBatis-Plus 的 API 形状，不含任何项目包名；未 import MP 的文件自动跳过以避免通用方法名（eq/in/set）误报。自带扫 0 文件拒过闸。

**落地后验证**：全量跑一遍。存量违规多时与团队确认策略：修复存量，或 `@LintIgnore` 豁免存量、新增零容忍。
