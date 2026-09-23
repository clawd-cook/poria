## 职责

进入即**复用skill `/openspec-propose` 完全按照skill内部流程执行**
如果找不到skill直接退出，提示用户。

调 `python3 .workflow/engine/cli.py project-type` 取项目类型，读对应专属补充：
- `backend` → 读 `./backend.md`
- `frontend` → 读 `./frontend.md`

## 出口

`advance <task> --step workflow-propose`，照 CLI 打印的闸门指令当场接续。
