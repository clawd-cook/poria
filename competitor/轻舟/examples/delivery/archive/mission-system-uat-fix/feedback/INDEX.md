# Feedback Index — mission-system-uat-fix

由可报问题的节点登记（建档即落 severity，缺省 normal），feedback-loop 闭环时记 category（design/code/test 三分类），修完移到 resolved。

from = 拦截阶段；根因 = category（design/code/test 三分类，与沉淀去向对齐）。两轴构成漏点地图。

解决方式 = auto（AI 自己发现自己解决、全程没打断人）/ asked（为它问过人、等过人回话才搞定，含人自己动手改）。与 from 交叉：from=验证节点 + auto 才是完全自治。「仅记录」闭环的条目此列为空（没走解决路径）。

| id | from | 根因 | severity | status | 解决方式 | resolved_by |
|---|---|---|---|---|---|---|
| issue-1 | workflow-code-review | code | normal | resolved | auto | workflow-code-review 原地修 |
| issue-2 | workflow-code-review | code | normal | resolved | auto | workflow-code-review 原地修 |
