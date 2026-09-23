# 能力地图结构化镜像（`capability-map.json`）

> 本文件是 `SKILL.md` 里 `capability-map.json` 的完整 schema 与强约束。**凡本轮要动「产品域全景」表（冷启建表、细化把某域 planned→generated、增量增删域或改职责），动手前读本文，并在同一轮同步更新 json。**

`product-overview.md` 里的「产品域全景」是给人读的表格；线上系统需要机器可读的同一份数据，落成 `<产品名>/capability-map.json`。**它是全景表的孪生镜像**：全景表里列了几个域、各域什么中文名/职责/状态，json 就有几条、字段一一对应。

```json
{
  "product": "scrm-platform",
  "productName": "企微 SCRM 平台",
  "domains": [
    {
      "name": "用户引流（获客）",
      "responsibility": "通过各类活码与营销工具把消费者引流进企微号/群",
      "status": "generated",
      "doc": "用户引流（获客）/用户引流（获客）.md"
    },
    {
      "name": "用户触达",
      "responsibility": "群发、SOP、欢迎语、关键词回复、朋友圈、素材库",
      "status": "planned",
      "doc": null
    }
  ]
}
```

字段约定：

- `name`——能力域中文名，**同时是二级目录名和主键**，须唯一稳定。与全景表「能力域」列一致；`generated` 域的目录就叫这个名字。`planned` 域此时还没目录，`name` 就是将来建目录时要用的名字，**从 planned 转 generated 时 name 不变**（目录名即照此建）。
- `responsibility`——一句话职责，与全景表「一句话职责」列一致。
- `status`——`generated`（已细化、目录已存在）| `planned`（全景已列、尚未生成文档）。对应全景表里带 ✅ 链接的 vs 标「待生成」的。
- `doc`——细化文档相对路径（相对产品目录），中文路径，如 `用户引流（获客）/用户引流（获客）.md`；`planned` 时为 `null`。多篇时指向域概览或主文档，与全景表链接一致。

**强约束——json 与全景表强绑定，不允许只改一个**：凡改动「产品域全景」表（冷启建表、细化把某域从 planned 转 generated、增量增删域或改职责），**必须在同一轮内同步更新 `capability-map.json`**，并列入收尾校验：逐条核对 json 与全景表的域集合、中文名、职责、状态、目录名/链接是否一致。二者漂移视为未完成。
