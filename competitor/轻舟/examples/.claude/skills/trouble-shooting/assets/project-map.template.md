# 项目地图（Project Map）模板

> **使用方法**：把本文件复制为 `~/.claude/project-map.md`，然后：
> 1. 把下方「代码根目录」改成你本地存放仓库的目录；
> 2. 核对映射表里的「代码路径（相对根目录）」与你的实际目录结构是否一致，不一致的改掉；
> 3. 用不到的应用行可以删掉，遇到新应用时排查 skill 会引导你追加。

## 代码根目录

所有仓库的代码路径均相对于以下根目录。**请改成你自己存放仓库的目录**：

```
代码根目录 = /path/to/your/repos
```

下表「代码路径」列写的是相对于「代码根目录」的子路径，完整路径 = `代码根目录` + `/` + 子路径。

## 应用/仓库映射表

| 应用名 | 关键功能 | 代码路径（相对根目录） |
|---|---|---|
| wecom-scrm-platform | 进群领/裂变/金豆/企微告警/选品/官方群列表/官方群活码 | `wecom-scrm/wecom-scrm-platform` |
| wecom-scrm-platform-zd | 群发/账号管理/权限管理/群模板/群控 RPA | `wecom-scrm/wecom-scrm-platform-zd` |
| wecom-dmp | 企微官方数据异构（企微号、外部客户群聊、客户关系等） | `wecom-dmp` |
| wecom-growth-soa | 企微增长 SOA / 进群领、裂变、金豆 C 端用户操作 | `wecom-growth` |
| wecom-scrm-system | 企微 SCRM 系统管理（项目管理平台管理员功能） | `wecom-scrm/wecom-scrm-system` |
| wecom-scrm-base | 企微外部服务 RPC 封装（企微接口等） | `wecom-scrm/wecom-scrm-base` |
| wecom-scrm-broker | 企微回调通知变更 & 企微 token 管理中心 | `wecom-scrm-broker` |

## 配置文件读取规则

数据验证前先在 `<代码根目录>/<代码路径>` 下定位数据源，查找优先级：

1. **优先**：`opencli-config.md`，存在则直接用其中的数据源信息。
2. **次选**：在代码库中找`*.properties`或`*.yml`格式配置文件,不同环境（pre → `application-pre`；pro → `application-pro`和`application`），读其中数据源配置。

## 维护说明

- 命中映射表里没有的应用时，排查 skill 会询问其代码路径，确认后追加一行到本表。
- grep 代码或读配置时路径不存在，说明条目已过时，更新「代码路径」或「代码根目录」。
