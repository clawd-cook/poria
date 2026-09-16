# 京ME ↔ Claude Code 打通实战：从消息汇总到分支修复的全自动工作流

一套已验证可用的链路：早上汇总京ME消息 → 判断是否平台 bug → 拉最新代码 → 建修复分支 → 定位并修复。
全程在 Claude Code 会话里一句话驱动，无需切换工具。

一、背景与思路

**痛点**：每天京ME里大量群消息（告警、升级通知、业务方反馈），要人工翻消息、判断哪些和自己有关、是否是平台 bug，然后切到 IDE 拉分支排查。环节割裂。

**思路**：机器上已装 JoyClaw（京东内部 OpenClaw，`~/.joyclaw`），自带 `jmechat` channel，走 jme-auth SSO，**有完整京ME聊天读取权限*
*（网页版万能博士没有）。把它包成 MCP server 挂进 Claude Code，Claude Code 就能直接读京ME。

Claude Code 会话

↓ MCP (joyclaw 工具)

JoyClaw agent（小虾虾，joyme_* 工具全家桶）

↓ SSO

京ME 聊天 / 日程 / JoySpace 文档 / 慧记

二、环境准备

1. 前置条件

| 项 | 要求 |
| --- | --- |
| JoyClaw | 已安装且网关在跑（`~/.joyclaw`，版本 v2.5.6+） |
| 京ME客户端 | 已登录（JoyME 进程常驻） |
| Claude Code | 已安装（`claude -p` 可用） |

验证 JoyClaw 活着：

lsof -i :18810 -P -n | grep LISTEN

# 应看到 node ... 127.0.0.1:18810 (LISTEN)

2. 写 MCP 桥（约 66 行，一次到位）

创建 `~/mcp-joyclaw/server.mjs`：

#!/usr/bin/env node

// MCP server: JoyClaw bridge — exposes JoyClaw agent as tool for Claude Code

import { spawn } from 'node:child_process';

const NODE = process.env.HOME + '/.joyclaw/node/node-v22.16.0-darwin-arm64/bin/node';

const OC = process.env.HOME + '/.joyclaw/apps/v2.5.6/openclaw.mjs';

function runAgent(message, timeoutSec = 300) {

return new Promise((resolve) => {

const p = spawn(NODE, [OC, 'agent', '--agent', 'main', '--timeout', String(timeoutSec), '--json',
'--message', message], {

env: { ...process.env, NO_COLOR: '1' },

});

let out = '', err = '';

p.stdout.on('data', d => out += d);

p.stderr.on('data', d => err += d);

p.on('close', () => {

// --json 模式：stdout = 插件日志 + LLM 流 + 末尾最终 JSON {payloads:[{text}], meta}

const lines = out.split('\n');

let parsed = null;

for (let i = lines.length - 1; i >= 0; i--) {

if (lines[i] !== '{') continue;

try { parsed = JSON.parse(lines.slice(i).join('\n')); break; } catch {}

}

let text;

if (Array.isArray(parsed?.payloads)) {

text = parsed.payloads.map(p => p.text || '').filter(Boolean).join('\n');

} else if (parsed?.output?.content) {

text = parsed.output.content.map(c => c.text || '').filter(Boolean).join('\n');

} else {

text = out;

}

resolve({ text, raw: parsed });

});

});

}

const TOOLS = [

{

name: 'joyclaw',

description: '向本机 JoyClaw agent（小虾虾，可访问京ME聊天/日程/JoySpace文档/慧记等 joyme_* 工具）发送指令并返回结果。用于汇总京ME聊天记录、查询消息、
读文档等。',

inputSchema: {

type: 'object',

properties: {

message: { type: 'string', description: '给 JoyClaw 的任务指令，如：汇总我今天的京ME聊天记录' },

timeoutSec: { type: 'number', description: '超时秒数，默认 300' },

},

required: ['message'],

},

},

];

// JSON-RPC over stdio (MCP)

let buf = '';

process.stdin.on('data', chunk => {

buf += chunk;

let idx;

while ((idx = buf.indexOf('\n')) >= 0) {

const line = buf.slice(0, idx).trim();

buf = buf.slice(idx + 1);

if (!line) continue;

let msg;

try { msg = JSON.parse(line); } catch { continue; }

handle(msg);

}

});

function send(obj) {

process.stdout.write(JSON.stringify(obj) + '\n');

}

async function handle(msg) {

const { id, method, params } = msg;

if (method === 'initialize') {

send({ jsonrpc: '2.0', id, result: { protocolVersion: '2024-11-05', capabilities: { tools: {} },
serverInfo: { name: 'joyclaw-bridge', version: '0.1.0' } } });

} else if (method === 'notifications/initialized') {

// no response needed

} else if (method === 'tools/list') {

send({ jsonrpc: '2.0', id, result: { tools: TOOLS } });

} else if (method === 'tools/call') {

const args = params.arguments || {};

const { text } = await runAgent(args.message || '', args.timeoutSec || 300);

send({ jsonrpc: '2.0', id, result: { content: [{ type: 'text', text: String(text).slice(0, 100000) }
] } });

} else if (id !== undefined) {

send({ jsonrpc: '2.0', id, error: { code: -32601, message: 'method not found: ' + method } });

}

}

3. 注册进 Claude Code 全局配置

`~/.claude.json` 的 `mcpServers` 加：

"joyclaw": {

"command": "/Users/<你>/.joyclaw/node/node-v22.16.0-darwin-arm64/bin/node",

"args": ["/Users/<你>/mcp-joyclaw/server.mjs"]

}

4. 验证

新开 Claude Code 会话，直接问：

用 joyclaw 工具查询：我今天京ME里有哪些重要聊天消息？

能返回真实消息汇总即通。实测输出示例（脱敏）：

今日重点，按优先级：

1. 🔴 潜客查坑位接口 3002 报错（NoSuchFieldError: TECH）—— 待排查

2. 🔴 24 条告警（UWC 20 + 洞察 4）

3. 🟡 实验分析大改版：4 项修复已到准生产，待验证

4. ⚠️ 明天 14:00-16:30 JMQ 升级 finance-com-1

三、工作流实战

打通后，完整开发闭环变成**一段对话**。以下在同一个 Claude Code 会话里依次说：

Step 1：汇总当天京ME消息

汇总我今天的京ME消息，重点标出：报错、告警、@我的、待我验证的事项

Claude Code 自动调 joyclaw 工具 → JoyClaw 读京ME → 结构化汇总。

Step 2：判断是否平台 bug

第 1 条"潜客查位置接口 3002 报错 NoSuchFieldError: TECH"，

帮我判断是我们的问题还是平台 bug：

先查这个报错在哪些群出现、是否有人确认；

再搜我们仓库里 TECH 相关代码，看是不是我们引用了平台已下线的枚举/字段

判断信号（经验）：

- **平台 bug 特征**：多个互不相关的业务方群里同报；报错点在平台 SDK/网关层（如 NoSuchFieldError 指向平台包名）；升级公告时间与报错时间吻合
- **自己的问题**：只在我们的调用链路；代码里能 grep 到相关符号；最近的 commit 触碰过该模块
Step 3：获取最新分支 + 创建修复分支

确认是自己的问题后：

在 ~/IdeaProjects/JD/joycode 仓库：

切到 master 拉最新，然后从 master 创建修复分支

fix/tech-enum-error-20260915

分支命名规范：英文小写 + 日期后缀（团队约定），commit 信息中文。

Step 4：定位问题并修复

NoSuchFieldError: TECH 说明运行时类里没有这个字段。

定位：找到引用 TECH 的代码，确认平台 SDK 版本变更后该枚举是否被移除/改名。

修复：替换为当前 SDK 的等价枚举或改用接口返回值判断。

修完跑相关单测，自验证后提交。

Claude Code 用 grep/codegraph 定位 → 改代码 → 跑测试 → 生成中文 commit。

Step 5（可选）：结果回推京ME

把修复结论用 joyclaw 推送到京ME：告诉"实验分析大改版"群里

TECH 报错已定位为 SDK 枚举下线，修复分支 fix/tech-enum-error-20260915 已提交，待回归

四、进阶玩法

| 场景 | 一句话 |
| --- | --- |
| 每日站会材料 | "汇总我昨天京ME里项目相关的讨论，按主题分组" |
| 写 JoySpace 文档 | "把今天的排障过程整理成文档，用 joyclaw 存到 JoySpace" |
| 查日程 | "我本周有哪些日程" |
| 定时任务 | JoyClaw 自带 cron：`openclaw.mjs cron add --schedule "0 9 * * *" --message "..."` 每天早上自动汇总推送 |
| 读文档 | "joyclaw 读一下 JoySpace 上 xxx 文档，对比当前实现" |

五、踩坑记录

1. **网页版万能博士（JoySpace）读不了个人消息**——走企业知识检索，无 jme 权限。别在这条路上浪费时间。
2. **京ME客户端万能博士无法外部遥控**——在客户端进程内，无 API。JoyClaw 是唯一外部通路。
3. **JoyClaw CLI stdout 混插件日志**——必须加 `--json`，解析末尾 `payloads[].text`；裸文本模式会被日志污染。
4. **JoyClaw 网关必须常驻**——桥只是透传 CLI，网关挂了工具就超时。`lsof -i :18810` 检查。
5. **MCP 配置改完要重开会话**——当前会话不热加载。
六、架构小结

┌─────────────────────────────────────────────┐

│ Claude Code 会话（大脑：分析/决策/写代码）      │

│   工具: read/write/edit/exec/grep + joyclaw  │

└──────────────┬──────────────────────────────┘

│ MCP stdio (JSON-RPC)

┌──────────────┴──────────────────────────────┐

│ ~/mcp-joyclaw/server.mjs（薄桥，约66行）       │

└──────────────┬──────────────────────────────┘

│ openclaw CLI --json

┌──────────────┴──────────────────────────────┐

│ JoyClaw gateway（常驻 :18810）               │

│   agent: 小虾虾  channel: jmechat            │

│   tools: joyme_joychat/joyspace/joyminutes… │

└──────────────┬──────────────────────────────┘

│ jme-auth SSO

京ME · 日程 · 文档 · 慧记

核心原则：**Claude Code 做大脑，JoyClaw 做京ME的手脚，MCP 桥只做透传**。桥越薄越稳，所有智能放在两端。
