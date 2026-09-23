# propose 阶段 frontend 项目协议
openspec-propose 产物完成后，执行出口校验：

1. 读取 `openspec/config.yaml` 的 `schema` 字段（如 `fe-spec-driven`）
2. 检查 `openspec/schemas/<schema>/exit-rules.md` 是否存在
    - **存在** → 加载该文件，按其中定义的校验规则执行；不通过则修补后重验，不得跳过
    - **不存在** → 无额外校验，直接放行
