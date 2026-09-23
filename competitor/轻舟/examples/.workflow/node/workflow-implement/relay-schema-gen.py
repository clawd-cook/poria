#!/usr/bin/env python3
"""
relay-schema-gen: 把 relay `get_node_data` 返回的原始 Figma layerData
确定性地规范化成精简 schema，供静态稿 subagent 生成静态稿。

核心价值：relay layerData 原生带
Figma Auto Layout（layoutMode/itemSpacing/padding*/align*），直接翻成 flex，
不再让 LLM 从坐标反推——这是静态稿质量优于 deco 的关键。

推荐用法 - build（一步到位，编排层内聚）：
  python3 relay-schema-gen.py build --task <task> --node-id <nodeId> --out <schema.json>

  脚本**内部直连 relay HTTP MCP**（读 ~/.claude.json 的 mcpServers.zero-design
  url+headers），一条命令跑完 get_node_data(root) → list-masters → 逐个
  get_node_data(母版) → normalize，产出精简 schema + 摘要（node_count 等）。
  agent 无需逐个手调 MCP。MCP 不可达/鉴权失败 → 脚本软失败：回退提示手动
  prepare/normalize 流程（见下），agent 用自己的 MCP 会话逐步执行。

分步用法（build 回退路径 / 调试；MCP 调用由 agent 发起）：

阶段 1 - prepare:
  python3 relay-schema-gen.py prepare --task <task> --node-id <nodeId>

  从 context.md 读设计稿链接（取 id=designId）和 CDN 图片导出倍率，
  输出 get_node_data 调用参数 + 后续 normalize 命令提示。

  agent 拿到参数后调用 zero-design 的 get_node_data(designId, nodeId,
  includeChildrenData=true)。返回体通常很大（几百 K），MCP 会自动落盘到
  tool-results 路径并在返回里给出；小结果内联返回时 agent 需自行写盘。

阶段 2 - normalize:
  python3 relay-schema-gen.py normalize --raw <原始layerData落盘路径> --out <schema.json>

  读原始 layerData，确定性转成精简 schema（block/text/image + rect + style +
  sameLevel），写盘。体积从几百 K 压到几 K。
"""

import argparse
import json
import math
import os
import re
import sys
import urllib.error
import urllib.request
from urllib.parse import urlparse, parse_qs


SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.abspath(os.path.join(SCRIPT_DIR, '..', '..', '..'))

# 组件树 join 公共件（与 verify-tree.py 共用，同目录）——sys.path 保证 importlib 加载本文件时也能找到
sys.path.insert(0, SCRIPT_DIR)
from tree_util import parse_component_table, nearest_ancestor_in_set  # noqa: E402

# fontName.style → CSS font-weight
FONT_WEIGHT_MAP = {
    'thin': 100, 'extralight': 200, 'ultralight': 200, 'light': 300,
    'regular': 400, 'normal': 400, 'medium': 500, 'semibold': 600,
    'demibold': 600, 'bold': 700, 'extrabold': 800, 'ultrabold': 800,
    'black': 900, 'heavy': 900,
}

# 装饰类节点：不可直接用 DOM 语义还原，交静态稿 subagent + export_image/svg 定夺
DECORATIVE_TYPES = {'vector', 'booleanoperation', 'line', 'arrowline', 'ellipse'}
CONTAINER_TYPES = {'frame', 'group'}

# 自有字体（花字/艺术字）识别：relay 文字节点的 `fontName.family` 是解析后的真实字体名
# （如 'PingFang SC' 系统字体 vs '造字工房方黑体' 自有艺术字）；flat `fontFamily` 恒为哈希、无区分度。
# 判据用「移动端 web 安全字体白名单」：fontName.family 不在白名单 → 自有字体，浏览器加载不到、
# 直接渲染会回退系统字体走样。命中只作「候选」信号（customFont），由静态稿 subagent 目视决定
# 切图 vs 保持文字——白名单从宽命中（宁可多标候选，目视再定），因为漏标艺术字比多标更糟。
SYSTEM_FONT_FAMILIES = {
    'pingfang sc', 'pingfang tc', 'pingfang hk', 'pingfang', '苹方', '苹方 简', '苹方 繁',
    'helvetica', 'helvetica neue', 'arial', 'arial narrow', 'sans serif', 'serif', 'monospace',
    'system ui', 'apple system', 'blinkmacsystemfont', 'roboto', 'droid sans',
    'noto sans', 'noto sans sc', 'noto sans cjk sc', 'noto sans cjk',
    'heiti sc', 'heiti tc', 'stheiti', 'songti sc', 'stsong', 'kaiti', 'stkaiti',
    'microsoft yahei', '微软雅黑', 'simsun', '宋体', 'simhei', '黑体',
    'miui', 'mi lanting', 'oppo sans', 'harmonyos sans', 'harmony os sans', 'huawei sans',
    # 京东品牌字体：项目样式文件常规 @font-face 提供，浏览器可加载 → 按可用（非自有花字）处理
    '京东正黑', 'jdzhenght', 'jd zhenght', 'jdzhenghei', 'jd zhenghei',
}


def is_custom_font(node: dict) -> bool:
    fam = ((node.get('fontName') or {}).get('family') or '').strip()
    if not fam:
        return False
    # 归一化：小写 + 连字符/下划线→空格 + 折叠多空格（兼容 'PingFang-SC'/'PingFang SC' 分隔符变体）
    norm = re.sub(r'[\s_-]+', ' ', fam.lower()).strip()
    # 剥掉尾部版本号（'京东正黑 v2.3' / '… 2.2' → '京东正黑'），免逐版本维护白名单
    norm = re.sub(r'\s+v?\d[\d.]*$', '', norm).strip()
    return norm not in SYSTEM_FONT_FAMILIES


def read_file(path: str) -> str:
    full = os.path.join(PROJECT_ROOT, path) if not os.path.isabs(path) else path
    if not os.path.exists(full):
        print(f"ERROR: file not found: {full}", file=sys.stderr)
        sys.exit(1)
    with open(full, 'r', encoding='utf-8') as f:
        return f.read()


# ---------- relay HTTP MCP client（stdlib，供 build 一步取数）----------
# 只依赖标准库 urllib：这些脚本跑在用户仓库、很难升级，刻意不引入 mcp SDK / httpx
# 等第三方依赖（见 packages/workflow/CLAUDE.md「local 逻辑必须傻 + 全程软失败」）。
# Streamable HTTP 协议：POST JSON-RPC，先 initialize 拿 Mcp-Session-Id，再
# notifications/initialized（202/空体，不解析），最后 tools/call。响应体可能是
# application/json 或 text/event-stream，两者都处理。任一环失败抛 RelayMcpError，
# 由 cmd_build 兜住并回退到 agent 手动流程——绝不阻断。

RELAY_MCP_SERVER = 'zero-design'


class RelayMcpError(Exception):
    """relay MCP 直连失败（配置缺失/网络/鉴权/协议/解析）。build 捕获后软回退。"""


def _load_relay_mcp_config():
    """从 ~/.claude.json 读 mcpServers.zero-design 的 url + headers。"""
    path = os.path.expanduser('~/.claude.json')
    if not os.path.exists(path):
        raise RelayMcpError('未找到 ~/.claude.json，relay MCP 未配置')
    try:
        with open(path, encoding='utf-8') as f:
            conf = json.load(f)
    except (json.JSONDecodeError, OSError) as e:
        raise RelayMcpError(f'读取 ~/.claude.json 失败: {e}')
    cfg = (conf.get('mcpServers') or {}).get(RELAY_MCP_SERVER)
    if not isinstance(cfg, dict) or not cfg.get('url'):
        raise RelayMcpError(
            f'~/.claude.json 未配置 mcpServers.{RELAY_MCP_SERVER}.url'
            '（按 relay-mcp.md 跑 `lbcli relay get-token` 配置）'
        )
    return cfg['url'], dict(cfg.get('headers') or {})


class RelayMcpClient:
    """极简 Streamable HTTP MCP 客户端：一次 initialize、复用 session 连续 call_tool。"""

    def __init__(self, url=None, headers=None, timeout=180):
        if url is None:
            url, headers = _load_relay_mcp_config()
        self.url = url
        self.headers = headers or {}
        self.timeout = timeout
        self.session_id = None
        self.proto = '2025-06-18'
        self._rpc_id = 0
        self._initialized = False

    def _raw_post(self, payload):
        hdr = {
            'Content-Type': 'application/json',
            'Accept': 'application/json, text/event-stream',
            'MCP-Protocol-Version': self.proto,
        }
        hdr.update(self.headers)
        if self.session_id:
            hdr['Mcp-Session-Id'] = self.session_id
        data = json.dumps(payload).encode('utf-8')
        req = urllib.request.Request(self.url, data=data, headers=hdr, method='POST')
        try:
            resp = urllib.request.urlopen(req, timeout=self.timeout)
        except urllib.error.HTTPError as e:
            detail = ''
            try:
                detail = e.read().decode('utf-8', 'replace')[:200]
            except Exception:  # noqa: BLE001 — 读错误体本身失败无所谓，主因是 e.code
                pass
            if e.code in (401, 403):
                raise RelayMcpError(
                    f'relay MCP 鉴权失败（HTTP {e.code}）：token 缺失/过期。'
                    '按 relay-mcp.md 健康检查跑 `lbcli relay get-token` 刷新（首次登录/过期须 agent 侧完成）'
                    f'后重跑 build。{detail}'
                )
            raise RelayMcpError(f'relay MCP HTTP {e.code}: {detail}')
        except (urllib.error.URLError, OSError) as e:
            raise RelayMcpError(f'relay MCP 网络不可达: {e}')
        return resp.read().decode('utf-8', 'replace'), resp.headers

    @staticmethod
    def _parse_body(raw, headers):
        """application/json → 直接 loads；text/event-stream → 取末个带 result/error 的事件。
        空体（通知的 202 响应）返回 None。

        SSE：一个事件的 data 可拆成多行 `data:`（规范用 \\n 拼接），空行为事件边界——
        故按事件缓冲 data 行、边界处整体解析，别逐行 json.loads（否则被拆的 JSON 永远解析失败）。
        """
        ct = headers.get('Content-Type') or ''
        if 'text/event-stream' in ct:
            parsed = None
            buf = []

            def _flush():
                nonlocal parsed
                if not buf:
                    return
                try:
                    obj = json.loads('\n'.join(buf))
                except json.JSONDecodeError:
                    return
                if isinstance(obj, dict) and ('result' in obj or 'error' in obj):
                    parsed = obj

            for line in raw.splitlines():
                if line.startswith('data:'):
                    seg = line[5:]
                    buf.append(seg[1:] if seg.startswith(' ') else seg)  # 去掉冒号后至多一个前导空格
                elif line == '':          # 空行 = 事件边界
                    _flush()
                    buf.clear()
            _flush()                       # 末事件可能无尾随空行
            return parsed
        raw = raw.strip()
        if not raw:
            return None
        try:
            return json.loads(raw)
        except json.JSONDecodeError as e:
            raise RelayMcpError(f'relay MCP 响应解析失败: {e}; body[:120]={raw[:120]!r}')

    def _ensure_session(self):
        # 用独立 _initialized 标志 gate，不用 session_id——无状态服务端不回 Mcp-Session-Id 时
        # session_id 恒 None，若据此判会导致每次 call_tool 都重跑 initialize + 重发 initialized。
        if self._initialized:
            return
        self._rpc_id += 1
        raw, headers = self._raw_post({
            'jsonrpc': '2.0', 'id': self._rpc_id, 'method': 'initialize',
            'params': {
                'protocolVersion': self.proto,
                'capabilities': {},
                'clientInfo': {'name': 'relay-schema-gen', 'version': '1.0'},
            },
        })
        self.session_id = headers.get('Mcp-Session-Id')  # 有状态服务端回；无状态为 None（后续请求省略该头）
        result = self._parse_body(raw, headers)
        if isinstance(result, dict) and isinstance(result.get('result'), dict):
            neg = result['result'].get('protocolVersion')
            if neg:
                self.proto = neg
        # initialized 通知：服务端回 202 空体，不解析
        self._raw_post({'jsonrpc': '2.0', 'method': 'notifications/initialized'})
        self._initialized = True

    def call_tool(self, name, arguments):
        """调 MCP 工具，返回 result.content（list of {type,text}）。失败抛 RelayMcpError。"""
        self._ensure_session()
        self._rpc_id += 1
        raw, headers = self._raw_post({
            'jsonrpc': '2.0', 'id': self._rpc_id, 'method': 'tools/call',
            'params': {'name': name, 'arguments': arguments},
        })
        parsed = self._parse_body(raw, headers)
        if not isinstance(parsed, dict):
            raise RelayMcpError(f'relay MCP 工具 {name} 无有效响应')
        if 'error' in parsed:
            raise RelayMcpError(f'relay MCP 工具 {name} 返回错误: {parsed["error"]}')
        content = (parsed.get('result') or {}).get('content')
        if not isinstance(content, list):
            raise RelayMcpError(f'relay MCP 工具 {name} 响应缺 content')
        return content


def _make_relay_client():
    """构造 relay MCP 客户端。抽成工厂供单测注入 fake client。"""
    return RelayMcpClient()


def fetch_node_data(client, design_id, node_id, include_children):
    """调 get_node_data，返回 content（list [{type:text,text:<layerData json>}]）——
    与 MCP 大返回落盘的外壳一致，直接喂 _unwrap_layerdata 即可。"""
    return client.call_tool('get_node_data', {
        'designId': design_id,
        'nodeId': node_id,
        'includeChildrenData': include_children,
    })


# ---------- prepare ----------

def extract_design_url(context_content: str) -> str:
    match = re.search(r'(https?://relay\.jd\.com/file/design\?[^\s\)]+)', context_content)
    if match:
        return match.group(1)
    print("ERROR: context.md 中未找到 relay 设计稿链接", file=sys.stderr)
    sys.exit(1)


def extract_design_id(design_url: str) -> str:
    params = parse_qs(urlparse(design_url).query, keep_blank_values=True)
    if 'id' in params and params['id']:
        return params['id'][0]
    print(f"ERROR: 设计稿链接缺少 id 参数（designId）: {design_url}", file=sys.stderr)
    sys.exit(1)


def extract_cdn_ratio(context_content: str) -> str:
    match = re.search(r'CDN\s*(?:图片导出)?倍率[：:]\s*(\d+)', context_content)
    return match.group(1) if match else '2'


def cmd_prepare(args):
    context_content = read_file(f'delivery/{args.task}/context.md')
    design_url = extract_design_url(context_content)
    design_id = extract_design_id(design_url)
    ratio = extract_cdn_ratio(context_content)

    result = {
        'designId': design_id,
        'nodeId': args.node_id,
        'includeChildrenData': True,
        'ratio': ratio,
        'next': {
            'tool': 'zero-design get_node_data',
            'args': {'designId': design_id, 'nodeId': args.node_id, 'includeChildrenData': True},
            'note': (
                '返回体通常很大，MCP 会自动落盘并在返回里给出 tool-results 路径；'
                '把该路径作为下一步 normalize 的 --raw 传入。小结果内联返回时先自行写盘再 normalize。'
            ),
            'normalize_cmd': (
                f'python3 .workflow/node/workflow-implement/relay-schema-gen.py '
                f'normalize --raw <get_node_data落盘路径> --out <schema.json路径>'
            ),
        },
    }
    print(json.dumps(result, ensure_ascii=False, indent=2))


# ---------- normalize helpers ----------

def _num(v):
    """尽量转成 int（整数值去掉 .0），否则保留 float，None 原样。"""
    if v is None:
        return None
    if isinstance(v, bool):
        return v
    if isinstance(v, (int, float)):
        return int(v) if float(v).is_integer() else round(float(v), 2)
    return v


def rgb_to_css(color: dict, opacity=1.0):
    """Figma color（本 relay 变体 r/g/b 为 0-255 整数，也兼容 0-1 浮点）→ hex 或 rgba。"""
    if not color:
        return None
    r, g, b = color.get('r', 0), color.get('g', 0), color.get('b', 0)
    # 判定量纲：任一分量 > 1 视为 0-255
    if max(r, g, b) <= 1:
        r, g, b = r * 255, g * 255, b * 255
    r, g, b = int(round(r)), int(round(g)), int(round(b))
    if opacity is not None and opacity < 1:
        return f'rgba({r}, {g}, {b}, {round(opacity, 2)})'
    return '#{:02x}{:02x}{:02x}'.format(r, g, b)


def first_visible_fill(node: dict, fill_type: str = None):
    for f in node.get('fills') or []:
        if f.get('visible') is False:
            continue
        if fill_type is None or f.get('type') == fill_type:
            return f
    return None


def has_image_fill(node: dict) -> bool:
    return first_visible_fill(node, 'IMAGE') is not None


def image_hash(node: dict):
    f = first_visible_fill(node, 'IMAGE')
    return f.get('imageHash') if f else None


def first_visible_gradient(node: dict):
    """首个可见的渐变填充（LINEAR/RADIAL/ANGULAR），无则 None。"""
    for f in node.get('fills') or []:
        if f.get('visible') is False:
            continue
        if str(f.get('type', '')).startswith('GRADIENT'):
            return f
    return None


def _linear_gradient_angle(handles):
    """gradientHandlePositions[0]→[1] 向量 → CSS linear-gradient 角度（deg，0=向上顺时针）。

    relay handle 是 0-1 归一化坐标、y 轴向下；CSS 0deg=to top、90deg=to right、180deg=to bottom。
    故 angle = atan2(dx, -dy)。取不到有效两点时返回 None（调用方省略角度，退回 CSS 默认 to bottom）。
    """
    if not handles or len(handles) < 2:
        return None
    x0, y0 = handles[0].get('x', 0), handles[0].get('y', 0)
    x1, y1 = handles[1].get('x', 0), handles[1].get('y', 0)
    dx, dy = x1 - x0, y1 - y0
    if dx == 0 and dy == 0:
        return None
    return int(round((math.degrees(math.atan2(dx, -dy)) + 360) % 360))


def gradient_to_css(paint: dict):
    """Figma 渐变填充 → CSS gradient 字符串。

    - stop 颜色的 alpha 取自 `stop.color.a`（这套 relay 数据里透明度落在每个 stop 上，
      不是 paint 级），再乘 paint 级 `opacity`。
    - LINEAR 用 gradientHandlePositions 精确算角度；RADIAL/ANGULAR 暂走无方向兜底。
    分母不全好过分母错值：拿不到 stops 或类型不认时返回 None、由调用方跳过。
    """
    stops = paint.get('gradientStops') or []
    if not stops:
        return None
    paint_op = paint.get('opacity', 1)
    if paint_op is None:
        paint_op = 1
    pieces = []
    for s in stops:
        col = s.get('color') or {}
        a = col.get('a', 1)
        eff = round((a if a is not None else 1) * paint_op, 3)
        css = rgb_to_css(col, eff)
        if not css:
            continue
        pos = s.get('position')
        if isinstance(pos, (int, float)):
            pieces.append(f"{css} {round(_num(pos) * 100)}%")
        else:
            pieces.append(css)
    if not pieces:
        return None
    stop_str = ', '.join(pieces)
    t = paint.get('type')
    if t == 'GRADIENT_LINEAR':
        angle = _linear_gradient_angle(paint.get('gradientHandlePositions'))
        prefix = f'{angle}deg, ' if angle is not None else ''
        return f'linear-gradient({prefix}{stop_str})'
    if t == 'GRADIENT_RADIAL':
        return f'radial-gradient({stop_str})'
    if t == 'GRADIENT_ANGULAR':
        return f'conic-gradient({stop_str})'
    return None


def border_radius(node: dict):
    # 不规则形状（vector/booleanOperation/line/star/polygon）的真实几何是矢量路径，
    # topLeftRadius 等 corner-radius 字段是残留元数据、不描述可渲染形状——例如带尖角的
    # 「券已抵」气泡是 vector（名「矩形 50」）却仍带 radius:40。这些值被当 CSS 圆角叠到
    # 切图 <img> 上会二次裁剪、把尖角削成胶囊。不规则类型一律不读 radius（形状以切图像素为准）。
    if (node.get('type') or '').lower() in IRREGULAR_SHAPE_TYPES:
        return None
    keys = ['topLeftRadius', 'topRightRadius', 'bottomRightRadius', 'bottomLeftRadius']
    vals = [node.get(k) or 0 for k in keys]
    if not any(vals):
        return None
    if len(set(vals)) == 1:
        return _num(vals[0])
    return ' '.join(str(_num(v)) + 'px' for v in vals)


def border_style(node: dict):
    stroke = None
    for s in node.get('strokes') or []:
        if s.get('visible') is False:
            continue
        if s.get('type') == 'SOLID':
            stroke = s
            break
    if not stroke:
        return None
    w = node.get('strokeWeight') or 1
    color = rgb_to_css(stroke.get('color'), stroke.get('opacity', 1))
    return f'{_num(w)}px solid {color}'


def box_shadow(node: dict):
    parts = []
    for e in node.get('effects') or []:
        if e.get('visible') is False:
            continue
        if e.get('type') not in ('DROP_SHADOW', 'INNER_SHADOW'):
            continue
        off = e.get('offset') or {}
        x, y = _num(off.get('x', 0)), _num(off.get('y', 0))
        blur = _num(e.get('radius', 0))
        spread = _num(e.get('spread', 0))
        color = rgb_to_css(e.get('color'), (e.get('color') or {}).get('a', e.get('opacity', 1)))
        inset = 'inset ' if e.get('type') == 'INNER_SHADOW' else ''
        parts.append(f'{inset}{x}px {y}px {blur}px {spread}px {color}'.replace('  ', ' '))
    return ', '.join(parts) if parts else None


def font_weight(style):
    if not style:
        return None
    return FONT_WEIGHT_MAP.get(str(style).lower().replace(' ', ''))


def line_height_css(lh):
    if lh is None:
        return None
    if isinstance(lh, (int, float)):
        return f'{_num(lh)}px'
    if isinstance(lh, dict):
        unit, value = lh.get('unit'), lh.get('value')
        if value is None:
            return None
        if unit == 'PIXELS':
            return f'{_num(value)}px'
        if unit == 'PERCENT':
            return f'{_num(value)}%'
    return None


def text_segments(node: dict):
    """提取单个文本节点内的**逐字符混排字号**（Figma richTextStyle）。

    价格/面额这类文本常是「¥小 + 整数大 + 小数小」的混排——信息在 `richTextStyle`
    的 range 分段里，而 node 级 `fontSize` 只是个标量（甚至不匹配任一分段）。不提取
    就会被压平成单一字号（#5 价格字号错的根因）。

    返回 [{text, fontSize[, fontWeight]}...]（≥2 段且字号有差异时），否则 None（单一字号，
    走 node 级 fontSize 即可，不产 segments 保持精简）。

    **每段都带显式 fontSize**：run 未覆写 fontSize 时兜底到节点级 `fontSize`（价格里
    只有整数段覆写、`¥`/小数段继承节点值的情况很常见）。若不兜底，继承段会缺 fontSize，
    下游「逐段渲染各带自己 fontSize」时落到浏览器默认值，字号层级仍被打乱。
    """
    rts = node.get('richTextStyle')
    chars = node.get('characters')
    if not isinstance(rts, list) or len(rts) < 2 or not isinstance(chars, str):
        return None
    node_fs = node.get('fontSize')
    segs = []
    for run in rts:
        rng = run.get('range') or {}
        start, end = rng.get('start'), rng.get('end')
        if not isinstance(start, int) or not isinstance(end, int):
            return None  # 结构异常，放弃分段（退回单一字号）
        piece = {'text': chars[start:end]}
        fs = run.get('fontSize')
        if fs is None:
            fs = node_fs  # 未覆写 → 兜底节点级字号
        if fs is not None:
            piece['fontSize'] = f"{_num(fs)}px"
        fw = font_weight((run.get('fontName') or {}).get('style'))
        if fw:
            piece['fontWeight'] = fw
        segs.append(piece)
    # 所有分段字号相同 → 无混排，不产 segments
    sizes = {s.get('fontSize') for s in segs}
    if len(sizes) <= 1:
        return None
    return segs


def layout_style(node: dict) -> dict:
    """Auto Layout → flex；ABSOLUTE / 无 layoutMode → 兜底走定位。"""
    style = {}
    positioning = node.get('layoutPositioning')
    mode = node.get('layoutMode')

    if positioning == 'ABSOLUTE':
        style['position'] = 'absolute'
        style['left'] = f"{_num(node.get('x', 0))}px"
        style['top'] = f"{_num(node.get('y', 0))}px"
        return style

    if mode in ('VERTICAL', 'HORIZONTAL'):
        style['display'] = 'flex'
        style['flexDirection'] = 'column' if mode == 'VERTICAL' else 'row'
        if node.get('itemSpacing'):
            style['gap'] = f"{_num(node['itemSpacing'])}px"
        pads = {
            'paddingTop': node.get('paddingTop'),
            'paddingRight': node.get('paddingRight'),
            'paddingBottom': node.get('paddingBottom'),
            'paddingLeft': node.get('paddingLeft'),
        }
        for k, v in pads.items():
            if v:
                style[k] = f'{_num(v)}px'
        primary = node.get('primaryAxisAlignItems')
        counter = node.get('counterAxisAlignItems')
        justify_map = {'MIN': 'flex-start', 'CENTER': 'center', 'MAX': 'flex-end',
                       'SPACE_BETWEEN': 'space-between'}
        align_map = {'MIN': 'flex-start', 'CENTER': 'center', 'MAX': 'flex-end',
                     'BASELINE': 'baseline', 'STRETCH': 'stretch'}
        if primary in justify_map:
            style['justifyContent'] = justify_map[primary]
        if counter in align_map:
            style['alignItems'] = align_map[counter]
        if node.get('layoutWrap') == 'WRAP':
            style['flexWrap'] = 'wrap'
    return style


def self_align_style(node: dict) -> dict:
    """子节点相对父 flex 的自身对齐/拉伸。

    relay 的 `layoutAlign` / `layoutGrow` 描述「本节点在父 Auto Layout 交叉轴/主轴上
    的表现」——父 counterAxisAlignItems 只是默认值，子节点可用 layoutAlign 覆盖。
    静态稿若只照父的 alignItems、丢掉子的 layoutAlign，固定尺寸的 badge/标签会被父
    容器拉伸（如「自营」标从一行被拉成两行）。这里把子自身对齐显式翻成 alignSelf/flexGrow。
    """
    style = {}
    if node.get('layoutPositioning') == 'ABSOLUTE':
        return style
    align = node.get('layoutAlign')
    # INHERIT = 跟随父默认，无需显式输出；其余值是子节点的显式覆盖，必须保留
    align_map = {'STRETCH': 'stretch', 'CENTER': 'center', 'MIN': 'flex-start', 'MAX': 'flex-end'}
    if align in align_map:
        style['alignSelf'] = align_map[align]
    grow = node.get('layoutGrow')
    if isinstance(grow, (int, float)) and grow and grow > 0:
        style['flexGrow'] = _num(grow)
    return style


def size_style(node: dict, parent_mode: str = None) -> dict:
    """把 relay 的固定尺寸（`*AxisSizingMode: FIXED`）显式翻成 `style.width` / `style.height`。

    原则：**normalize 尽量完整、不丢权威值，生成阶段才不用猜**。`FIXED` 是设计的确定
    尺寸，对宽高一样权威，两轴对称翻。以前只翻 `layoutAlign:STRETCH → alignSelf`，漏了
    sizingMode 的 FIXED——固定尺寸只落进 `rect.*`；而静态稿被要求「照抄 style、禁止从 rect
    反推」，于是固定尺寸容器塌成内容尺寸（券条 50px 高塌陷、背景不撑满，问题 #4）。

    轴映射（按节点自身 layoutMode）：
      - HORIZONTAL：primaryAxisSizingMode→宽、counterAxisSizingMode→高
      - VERTICAL  ：primaryAxisSizingMode→高、counterAxisSizingMode→宽

    护栏（排除该节点在**父容器**里的弹性轴，避免和 flexGrow/alignSelf 打架）：
      - 沿父主轴伸展（`layoutGrow>0`）→ 父主轴那一维不钉（留给 flexGrow）
      - 沿父交叉轴拉伸（`layoutAlign==STRETCH`）→ 父交叉轴那一维不钉（留给 alignSelf:stretch）
    父主轴/交叉轴由 `parent_mode` 决定（HORIZONTAL：主轴=宽、交叉=高；VERTICAL 反之）。
    动态文本容器在 relay 本就是 `AUTO`（hug）不是 FIXED，不会被误钉导致文案变长溢出。
    根节点（parent_mode 为空）照实翻，root 的响应式（固定宽→100%）由 rewrite 阶段决定。
    """
    if node.get('layoutPositioning') == 'ABSOLUTE':
        return {}
    mode = node.get('layoutMode')
    if mode not in ('HORIZONTAL', 'VERTICAL'):
        return {}
    prim = node.get('primaryAxisSizingMode')
    cnt = node.get('counterAxisSizingMode')
    if mode == 'HORIZONTAL':
        width_fixed = prim == 'FIXED'
        height_fixed = cnt == 'FIXED'
    else:  # VERTICAL
        height_fixed = prim == 'FIXED'
        width_fixed = cnt == 'FIXED'

    grow = node.get('layoutGrow')
    grows = isinstance(grow, (int, float)) and grow > 0
    stretches = node.get('layoutAlign') == 'STRETCH'
    if parent_mode == 'HORIZONTAL':
        if grows:       # 沿父主轴(横)伸展 → 宽弹性
            width_fixed = False
        if stretches:   # 沿父交叉轴(纵)拉伸 → 高弹性
            height_fixed = False
    elif parent_mode == 'VERTICAL':
        if grows:       # 沿父主轴(纵)伸展 → 高弹性
            height_fixed = False
        if stretches:   # 沿父交叉轴(横)拉伸 → 宽弹性
            width_fixed = False

    style = {}
    w, h = node.get('width'), node.get('height')
    if width_fixed and isinstance(w, (int, float)) and w > 0:
        style['width'] = f'{_num(w)}px'
    if height_fixed and isinstance(h, (int, float)) and h > 0:
        style['height'] = f'{_num(h)}px'

    # hug(AUTO) 容器的尺寸下限：AUTO 轴不钉死 width/height（保留 hug 弹性），但把设计 rect
    # 的确定尺寸翻成 `minWidth`/`minHeight`。根因：relay 的 hug 尺寸是「按设计字体度量包裹内容」
    # 的确定结果，浏览器换字体后度量偏大就换行/溢出（气泡「3个」被挤成两行即此）。用 min-* 而非
    # width：内容更多时仍能撑大（hug 语义不丢），内容少时有下限不塌到换行。仅 NO_WRAP（换行容器
    # 本就该随内容多行、不设死下限）、非弹性轴（复用上面护栏结果）、rect 正尺寸时加。
    if node.get('layoutWrap') != 'WRAP':
        # 未被父弹性接管的轴（复用上面护栏语义）才加尺寸下限
        w_is_flex = (parent_mode == 'HORIZONTAL' and grows) or (parent_mode == 'VERTICAL' and stretches)
        h_is_flex = (parent_mode == 'VERTICAL' and grows) or (parent_mode == 'HORIZONTAL' and stretches)
        if 'width' not in style and not w_is_flex and isinstance(w, (int, float)) and w > 0:
            style['minWidth'] = f'{_num(w)}px'
        if 'height' not in style and not h_is_flex and isinstance(h, (int, float)) and h > 0:
            style['minHeight'] = f'{_num(h)}px'

    # 固定/确定尺寸子在父 flex 主轴上不收缩：relay 无「收缩」概念，CSS flex 子默认 flex-shrink:1，
    # 于是设计里定宽的固定子（按钮列/切图分隔/图标）紧邻 flexGrow 兄弟时被挤到声明尺寸以下、内容
    # 换行错位（「待发3个」被挤换行即此）。本节点在父主轴方向有确定尺寸（width/minWidth 或
    # height/minHeight，视父主轴而定）且非自身沿主轴 grow → 输出 flexShrink:0。
    has_w = 'width' in style or 'minWidth' in style
    has_h = 'height' in style or 'minHeight' in style
    if parent_mode == 'HORIZONTAL' and has_w and not grows:
        style['flexShrink'] = 0
    elif parent_mode == 'VERTICAL' and has_h and not grows:
        style['flexShrink'] = 0
    return style


def visual_style(node: dict, type_out: str) -> dict:
    style = {}
    # 背景 / 颜色
    solid = first_visible_fill(node, 'SOLID')
    if solid:
        css = rgb_to_css(solid.get('color'), solid.get('opacity', 1))
        if css:
            if type_out == 'text':
                style['color'] = css
            else:
                style['background'] = css
    elif type_out != 'text':
        # 无 SOLID 的非文字节点：尝试渐变背景（纯渐变头图/票券底等，否则整块底色丢失）。
        # 只覆盖「无 SOLID」这一情形，不动既有 SOLID 行为 → 零回归；
        # SOLID+渐变混合填充的图层叠放顺序语义未定，暂不处理（继续走上面的 SOLID）。
        grad = first_visible_gradient(node)
        if grad and grad.get('opacity', 1) != 0:
            gcss = gradient_to_css(grad)
            if gcss:
                style['background'] = gcss
    # 容器带图片填充 → 背景图（占位 imageHash）
    if type_out == 'block' and has_image_fill(node):
        style['backgroundImage'] = {'imageHash': image_hash(node)}

    radius = border_radius(node)
    if radius is not None:
        style['borderRadius'] = f'{radius}px' if isinstance(radius, (int, float)) else radius
    border = border_style(node)
    if border:
        style['border'] = border
    shadow = box_shadow(node)
    if shadow:
        style['boxShadow'] = shadow
    if node.get('clipsContent'):
        style['overflow'] = 'hidden'

    # 文字样式
    if type_out == 'text':
        if node.get('fontSize'):
            style['fontSize'] = f"{_num(node['fontSize'])}px"
        fam = (node.get('fontName') or {}).get('family')
        if fam:
            style['fontFamily'] = fam
        fw = font_weight((node.get('fontName') or {}).get('style'))
        if fw:
            style['fontWeight'] = fw
        lh = line_height_css(node.get('lineHeight'))
        if lh:
            style['lineHeight'] = lh
    return style


def _iter_subtree(node: dict):
    yield node
    for c in node.get('children') or []:
        if isinstance(c, dict):
            yield from _iter_subtree(c)


def is_decorative_subtree(node: dict) -> bool:
    """子树内无 text、无 image 填充 → 纯装饰（图标/线条/形状组），应整体切图。"""
    for d in _iter_subtree(node):
        if (d.get('type') or '').lower() == 'text':
            return False
        if first_visible_fill(d, 'IMAGE'):
            return False
    return True


def has_nested_instance(node: dict) -> bool:
    """实例子树里是否还嵌套别的组件实例（"组件套组件"）。

    嵌套实例意味着几何/布局分散在多层母版里，静态稿阶段无法只靠本次 layerData
    递归还原，应整体切图占位（数据由改写阶段接项目现成组件绑定）。
    """
    for c in node.get('children') or []:
        for d in _iter_subtree(c):
            if (d.get('type') or '').lower() == 'instance':
                return True
    return False


# 天然异形（无法用 CSS 圆角矩形还原）的图层类型
IRREGULAR_SHAPE_TYPES = {'vector', 'booleanoperation', 'line', 'arrowline', 'star', 'polygon'}


def shape_hint(node: dict) -> str:
    """判定装饰形状的复杂度，驱动 normalize 的 renderer 路由。

    返回 'complex'（异形 → 标 renderer:image、免目视直接切图）或
    'simple'（规整矩形/椭圆外壳、内容未知 → 标 renderer:image-candidate、留待静态稿目视）。

    误判为 simple 会把气泡尾、尖角、多缺口等异形放进 image-candidate、可能被 CSS 圆角矩形
    近似掉（如「券已抵X元」气泡）。判定从严：只有确凿的纯圆角矩形/正圆才算 simple，
    拿不准一律 complex。
    """
    t = (node.get('type') or '').lower()
    # 天然异形类型：一律切图
    if t in IRREGULAR_SHAPE_TYPES:
        return 'complex'
    # booleanOperation 常表现为 type 含 'boolean'
    if 'boolean' in t or 'vector' in t:
        return 'complex'
    # 非均匀圆角（四角不等）→ 气泡/票券缺口等异形
    keys = ['topLeftRadius', 'topRightRadius', 'bottomRightRadius', 'bottomLeftRadius']
    radii = [node.get(k) or 0 for k in keys]
    if len(set(radii)) > 1:
        return 'complex'
    # 带描边 + 圆角组合，或有阴影/多重填充 → 视觉复杂，倾向切图
    visible_strokes = [s for s in (node.get('strokes') or []) if s.get('visible') is not False]
    visible_fills = [f for f in (node.get('fills') or []) if f.get('visible') is not False]
    if visible_strokes and any(radii):
        return 'complex'
    if len(visible_fills) > 1:
        return 'complex'
    # 剩下：矩形/椭圆/frame 的纯色单填充、均匀圆角 → 可 CSS 还原
    if t in ('rectangle', 'ellipse') or t in CONTAINER_TYPES:
        return 'simple'
    # 未知形状从严
    return 'complex'


def classify(node: dict) -> tuple:
    """返回 (type_out, render_hint, atomic, expand_kind)。

    - render_hint 非 None → 需视觉判定（切图 vs CSS）
    - atomic True → 不再递归子节点（作为一个整体单元）
    - expand_kind → 组件实例的处置类别：'cut'（切图占位）| 'box'（递归内联真实结构）；
      非实例恒为 None
    """
    t = (node.get('type') or '').lower()
    has_children = bool(node.get('children'))

    if t == 'text':
        return 'text', None, True, None
    if t in DECORATIVE_TYPES:
        return 'image', 'image-candidate', True, None
    if t == 'instance':
        # 组件实例三分：
        # - 纯装饰（子树无 text/image）→ cut：整块切图占位。
        # - 含内容 + 套实例（组件套组件，如动态券卡）→ expand：递归展开真实内部结构。
        #   文本/布局就在本次 layerData 里可读，装饰子各自 cut、扁平子实例走 box；
        #   内容不依赖母版，故缺母版也不降级、照常递归。
        # - 扁平含内容（如内联徽章）→ box：母版补容器默认后内联；无母版降级 cut。
        if is_decorative_subtree(node):
            return 'image', 'image-candidate', True, 'cut'
        if has_nested_instance(node):
            return 'block', None, False, 'expand'
        return 'block', None, False, 'box'
    if t == 'rectangle':
        if has_image_fill(node):
            return 'image', None, True, None
        return 'block', None, False, None
    if t in CONTAINER_TYPES:
        if has_image_fill(node) and not has_children:
            return 'image', None, True, None
        # 纯装饰/图标子树（无 text/image）→ 整体切图，不拆内部形状
        if has_children and is_decorative_subtree(node):
            return 'image', 'image-candidate', True, None
        return 'block', None, False, None
    # 未知类型：有图片填充按图片，否则块
    if has_image_fill(node):
        return 'image', None, True, None
    return 'block', None, False, None


# 母版可继承给实例根的「容器级默认」属性：实例缺失（None/不存在）时才取母版。
# 只含布局/内边距/圆角/尺寸这类结构默认——刻意不含 visible/id/type/x/y/name/fills/strokes：
# 那些是实例的就位实测值或身份，母版是隐藏定义态（常 visible:false、定位 0,0、fills 为定义色），
# 继承会污染（如把母版 visible:false 带进来导致节点被裁）。
MASTER_INHERIT_PROPS = (
    'layoutMode', 'itemSpacing', 'counterAxisSpacing', 'layoutWrap',
    'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
    'primaryAxisAlignItems', 'counterAxisAlignItems',
    'primaryAxisSizingMode', 'counterAxisSizingMode',
    'topLeftRadius', 'topRightRadius', 'bottomRightRadius', 'bottomLeftRadius',
    'width', 'height',
)


def merge_master_defaults(inst: dict, master: dict) -> dict:
    """把母版根的容器级默认属性补进实例根（仅 MASTER_INHERIT_PROPS 白名单、仅补实例缺的）。

    浅合并：只补实例根节点；children 保持实例的（页面就位实测值，不用母版占位子树）。
    背景：扁平组件实例（badge/服务标）的 padding/圆角/layout/固定尺寸常只在母版里，
    实例根为 None——只靠实例数据会渲染成无内距、方角、非 flex 的残缺盒子。
    """
    merged = dict(inst)
    for k in MASTER_INHERIT_PROPS:
        if merged.get(k) is None and master.get(k) is not None:
            merged[k] = master[k]
    return merged


# 子孙几何补拉字段：在根级白名单基础上加 x/y。根级 merge 刻意排除 x/y（实例根有真实就位坐标，
# 母版是 0,0 定义态、继承会污染）；但**深层嵌套子孙**的 x/y 在实例 layerData 里缺失（几何在母版子树），
# 此时母版子的 x/y 才是真值——故子孙补拉含 x/y，且仅补实例缺（None）的字段、实例有值则 override 优先。
BACKFILL_GEOM_FIELDS = MASTER_INHERIT_PROPS + ('x', 'y')


def build_full_geom(full_masters: list) -> dict:
    """从「带 children 预取的母版」子树建 {母版子节点 id: 几何/布局字段} 映射。

    key = 母版子树内节点 id（= 实例子孙的 `refId`，全局唯一、跨母版不冲突）。
    供 normalize 对实例子孙按 refId 回填缺失几何（见 backfill_geom_by_ref）。
    """
    geom = {}

    def walk(n):
        if not isinstance(n, dict):
            return
        nid = n.get('id')
        if nid:
            g = {f: n[f] for f in BACKFILL_GEOM_FIELDS if n.get(f) is not None}
            if g:
                geom[nid] = g
        for c in n.get('children') or []:
            walk(c)

    for m in full_masters:
        walk(m)
    return geom


def backfill_geom_by_ref(node: dict, full_geom: dict) -> dict:
    """实例子孙自身缺几何时，按 `refId` 从母版全量几何兜底（仅补实例缺的字段）。

    只补几何/布局（BACKFILL_GEOM_FIELDS），不碰 visible/fills/身份——守 childless 的避污染纪律。
    实例已有值（override）优先，母版几何只填 None 空位。
    """
    ref = node.get('refId')
    if not ref or ref not in full_geom:
        return node
    merged = dict(node)
    for k, v in full_geom[ref].items():
        if merged.get(k) is None:
            merged[k] = v
    return merged



def _px_val(v):
    """'14px' / 14 / '14' → 14.0；无法解析 → None。"""
    if isinstance(v, (int, float)):
        return float(v)
    if isinstance(v, str):
        m = re.match(r'^([\d.]+)', v.strip())
        if m:
            try:
                return float(m.group(1))
            except ValueError:
                return None
    return None


def inline_badge_wrap_hint(node: dict, children_out: list) -> bool:
    """判定「内联徽章 + 文字绕排」布局候选（如商品名前的「自营」实心标）。

    relay 把 [矮徽章 + 可换行长文本] 建成同一 flex 行的兄弟，纯 flex 做不出
    「标题绕排矮徽章」（首行接徽章、次行全宽绕到徽章下方）——徽章会被垂直居中在
    多行文本块旁边（正是本项要机判出来、交静态稿套绕排模板的场景）。

    机判信号（区别于纯徽章行/普通图文行，误报率低）：
      - 容器是 HORIZONTAL flex（flexDirection:row）
      - 恰好 1 个 flexGrow 文本子（可换行长文本，flexGrow 表示它要占满/换行）
      - ≥1 个非文本、单行高的矮徽章兄弟（rect.h ≤ 文本 fontSize × 2）
    服务标行（多徽章、无文本子）不满足「恰好 1 个 flexGrow 文本」→ 天然不误伤。
    """
    if (node.get('layoutMode') or '').upper() != 'HORIZONTAL':
        return False
    grow_texts = [
        c for c in children_out
        if c.get('type') == 'text' and (c.get('style') or {}).get('flexGrow')
    ]
    if len(grow_texts) != 1:
        return False
    t = grow_texts[0]
    t_fs = _px_val((t.get('style') or {}).get('fontSize')) or 14
    for c in children_out:
        if c is t or c.get('type') == 'text':
            continue
        bh = (c.get('rect') or {}).get('h') or 0
        if 0 < bh <= t_fs * 2:
            return True
    return False


def normalize_node(node: dict, same_level: int, parent_mode: str = None, masters: dict = None,
                   full_geom: dict = None):
    """递归规范化。返回精简节点 dict，或 None（被裁剪）。

    parent_mode：父节点的 layoutMode（HORIZONTAL/VERTICAL），供 size_style 判断
    本节点的 layoutGrow 是否沿父纵向主轴伸展（伸展则高度弹性、不钉死）。
    masters：{componentId: 母版根节点} 映射（编排器预取，见 list-masters）。box 实例
    据此补母版容器默认属性；缺失则该 box 降级为 cut（切图占位，像素安全软失败）。
    full_geom：{母版子节点 id: 几何/布局字段}（来自 --masters-full 带 children 的母版，见 build_full_geom）。
    深层嵌套实例的子孙在实例 layerData 里缺几何（几何在母版子树）——按 `refId` 从这里回填，
    否则 rect 退化 0,0,0,0、静态稿被迫估算坐标（金币错位/图标裁切即此，见 ui2code 提案）。
    """
    if not isinstance(node, dict):
        return None
    if node.get('visible') is False:
        return None
    # 几何回填须在读 w/h/x/y 之前——补上缺失的几何/布局字段，避免退化为 0
    if full_geom:
        node = backfill_geom_by_ref(node, full_geom)
    w, h = node.get('width'), node.get('height')
    if (w == 0 or h == 0) and node.get('type', '').lower() != 'text':
        return None

    type_out, render_hint, atomic, expand_kind = classify(node)

    # box/expand 实例：有母版 → 用母版根容器默认补齐(padding/圆角/layout/尺寸)后再翻译、递归。
    # 无母版：box 降级为 cut（扁平实例缺容器默认会渲染成残缺盒子，切图占位软失败）；
    #        expand 不降级——其内容/布局在本次 layerData 里可读，靠 atomic=False 正常递归展开。
    if expand_kind in ('box', 'expand'):
        master = masters.get(node.get('componentId')) if (masters and node.get('componentId')) else None
        if master:
            node = merge_master_defaults(node, master)
            w, h = node.get('width'), node.get('height')       # 母版可能补了固定尺寸
        elif expand_kind == 'box':
            type_out, render_hint, atomic, expand_kind = 'image', 'image-candidate', True, 'cut'

    out = {
        'id': node.get('id'),
        'type': type_out,
    }
    name = node.get('name')
    if name:
        out['name'] = name
    x, y = node.get('x'), node.get('y')
    out['rect'] = {
        'x': _num(x if x is not None else 0),
        'y': _num(y if y is not None else 0),
        'w': _num(w or 0),
        'h': _num(h or 0),
    }

    style = {}
    style.update(layout_style(node))
    style.update(self_align_style(node))
    style.update(size_style(node, parent_mode))
    style.update(visual_style(node, type_out))
    if style:
        out['style'] = style

    if type_out == 'text':
        chars = node.get('characters')
        if chars is not None:
            out['text'] = chars
        segs = text_segments(node)
        if segs:
            out['segments'] = segs
        # 自有字体（花字/艺术字）候选：浏览器加载不到，静态稿须目视决定切图 vs 保持文字
        if is_custom_font(node):
            out['customFont'] = True
    if type_out == 'image':
        ih = image_hash(node)
        out['src'] = {'imageHash': ih} if ih else {'imageHash': None}
    # renderer：单一渲染判定轴——image = 当图渲染（真图 / 已定切图 / 异形装饰，免 §② 目视）；
    # image-candidate = 规整外壳、内容待目视（走 §② 判切图 vs CSS/inline-SVG）；缺省 = 普通元素（照抄 style）。
    if expand_kind == 'cut':
        renderer = 'image'                       # 装饰实例：已定切图
    elif render_hint == 'image-candidate':
        # 形状复杂度机判并入 renderer：complex（异形/描边圆角/多填充等）→ 直接当图切、免目视；
        # simple（规整矩形/椭圆外壳，内容未知）→ image-candidate 留待静态稿目视。
        renderer = 'image' if shape_hint(node) == 'complex' else 'image-candidate'
    elif type_out == 'image':
        renderer = 'image'                       # 真图（imageFill）
    else:
        renderer = None
    if renderer:
        out['renderer'] = renderer
    if expand_kind and node.get('componentId'):  # 组件实例（cut/box/expand）：保留 componentId 供改写判定复用
        # 不再输出 expandKind——box/expand 渲染动作一致，实例身份由 componentId + type/renderer 表达：
        # renderer:image=装饰切图；type:block+componentId=已内联/展开实例（照常渲染、别回母版/重拉）。
        out['componentId'] = node['componentId']
    out['sameLevel'] = same_level

    # atomic 单元（text / 实例 / 图标组 / 图片）不再递归——内部作为整体处理
    if not atomic:
        norm_children = []
        cur_mode = node.get('layoutMode')
        # 按 orderKey(fractional index) 字典序排序 children：relay 的 children 数组顺序不保证
        # 等于绘制序，真正权威是每个子的 orderKey（字典序越大 = 绘制越晚 = 视觉越上层）。
        # 排序后 sameLevel（下标）才真正等于绘制序，下游可安全用 z-index = sameLevel 翻译。
        # 加固：orderKey 非全覆盖——实例展开的母版子孙不带 orderKey（实测缺失总是整组一起缺、
        # 不与带 orderKey 的兄弟混在同一数组）。排序 key 用 (缺失标志, orderKey, 原下标)：
        #   ① 缺失的排最后而非最前（空串会排最前、把无序节点顶到底层，反而更危险）；
        #   ② 原下标兜底 → Python 稳定排序内同组保持原相对序；
        #   ③ 万一真出现「同数组混合」（假设被打破）→ 打 warning 留证据，不静默。
        raw_children = [c for c in (node.get('children') or []) if isinstance(c, dict)]
        def _ok(c):
            k = c.get('orderKey')
            return k if isinstance(k, str) and k else None
        n_with = sum(1 for c in raw_children if _ok(c) is not None)
        if 0 < n_with < len(raw_children):
            import sys as _sys
            print(f"[warn] node {node.get('id')} 的 children orderKey 部分缺失"
                  f"（{n_with}/{len(raw_children)} 有），绘制序可能不准", file=_sys.stderr)
        sorted_children = sorted(
            enumerate(raw_children),
            key=lambda t: (_ok(t[1]) is None, _ok(t[1]) or '', t[0]),
        )
        for _, child in sorted_children:
            nc = normalize_node(child, len(norm_children), cur_mode, masters, full_geom)
            if nc is not None:
                nc['sameLevel'] = len(norm_children)  # 裁剪后紧凑下标，相对绘制顺序不变
                norm_children.append(nc)
        if norm_children:
            out['children'] = norm_children
            # 内联徽章 + 文字绕排候选（矮徽章 + flexGrow 长文本同 flex 行）
            if inline_badge_wrap_hint(node, norm_children):
                out['layoutHint'] = 'inline-badge-wrap'

    return out


def _text_style_key(node: dict) -> tuple:
    """text 节点的样式指纹（字体/字号/字重/色/行高），用于判定两段文字是否同款。"""
    s = node.get('style') or {}
    return (s.get('fontFamily'), s.get('fontSize'), s.get('fontWeight'),
            s.get('color'), s.get('lineHeight'))


def apply_wrapped_text_split(root: dict) -> list:
    """后置单遍：以 inline-badge-wrap 行为种子，认回「被拆成多图层的绕排文字」。

    设计工具做不出「文字绕排 inline 徽章」，设计师把本应换行的整段文字拆成
    [徽章行内的第 1 行文本 A] + [父列里的同款续行文本 B…]。本 pass 把它们认回同一段：
    在承载该续行的纵向列上打 `layoutHint='wrapped-text-split'` + `wrappedTextGroup=[A,B…]`
    （A 在前、续行在后），供静态稿合并成**一个**数据绑定文本 + `line-clamp` 绕排徽章，
    而非当作「标题 + 副标题」两段分别渲染/绑定。

    **触发闸门 = badge-wrap 命中**（唯一种子）：既把开销限定在极少数 badge-wrap 点附近，
    又把「同款相邻文字=同一段」这一脆弱启发式锚在「确有徽章逼出拆分」的结构性理由上——
    无徽章逼迫则不合并（宁漏不误并合法的「名称+同款副标题」）。
    """
    splits = []

    def visit(node, ancestors):
        if node.get('layoutHint') == 'inline-badge-wrap':
            # 徽章行里的 flexGrow 文本 = 绕排段的第 1 行 A
            a = next((c for c in (node.get('children') or [])
                      if c.get('type') == 'text' and (c.get('style') or {}).get('flexGrow')), None)
            if a is not None:
                a_key = _text_style_key(a)
                # 由近及远找纵向列祖先，取第一个「直接子里有同款续行文本」的列
                for anc in reversed(ancestors):
                    if (anc.get('style') or {}).get('flexDirection') != 'column':
                        continue
                    conts = [c for c in (anc.get('children') or [])
                             if c.get('type') == 'text' and c is not a
                             and _text_style_key(c) == a_key]
                    if conts:
                        group = [a.get('id')] + [c.get('id') for c in conts]
                        anc['layoutHint'] = 'wrapped-text-split'
                        anc['wrappedTextGroup'] = group
                        splits.append({'column': anc.get('id'), 'group': group})
                        break
        for c in node.get('children') or []:
            visit(c, ancestors + [node])

    visit(root, [])
    return splits


def _unwrap_layerdata(data):
    """把 get_node_data 的三种外壳剥成裸节点 dict：
    ① MCP 大返回自动落盘格式 `[{"type":"text","text":"<json string>"}]`（编排器直接把 tool-results 路径喂进来时最常见）；
    ② `{data:{...}}` 包裹；③ 裸节点。返回节点（无法识别时原样返回，交调用方判类型）。
    历史坑：normalize 的 --raw/--masters/--masters-full 只认 ②，把 ① 当 list 静默跳过 →
    母版/几何回填全落空、深层实例 schema 退化成空 style（标题副标题塌等），且无报错。故统一走本 helper。"""
    if isinstance(data, list) and data and isinstance(data[0], dict) and 'text' in data[0]:
        try:
            data = json.loads(data[0]['text'])
        except (json.JSONDecodeError, TypeError):
            return data
    if isinstance(data, dict) and 'data' in data and isinstance(data['data'], dict) and 'id' in data['data']:
        data = data['data']
    return data


def _finalize_schema(schema: dict, raw_bytes: int, out, verbose: bool):
    """规范化后处理 + 落盘 + 组装摘要。normalize / build 共用（避免摘要逻辑双写漂移）。

    始终执行「绕排文字认回」单遍（会 mutate schema）；out 给定则写盘并回 (result, out_json)，
    否则回 (None, out_json)——调用方据此决定打印摘要还是打印 schema 本身。
    """
    # 后置单遍：认回被拆成多图层的绕排文字（以 badge-wrap 为种子）
    wrapped_splits = apply_wrapped_text_split(schema)

    # 汇总:box 实例(已内联,供改写按 componentId 复用)——供 --verbose 明细 / 计数摘要
    inlined_instances = []

    def _collect(n):
        if n.get('componentId') and n.get('type') == 'block':   # 已内联/展开的组件实例
            inlined_instances.append({'id': n.get('id'), 'name': n.get('name'), 'componentId': n.get('componentId')})
        for c in n.get('children') or []:
            _collect(c)

    _collect(schema)

    def _count_nodes(n):
        return 1 + sum(_count_nodes(c) for c in (n.get('children') or []))

    node_count = _count_nodes(schema)

    out_json = json.dumps(schema, ensure_ascii=False, indent=2)
    if not out:
        return None, out_json

    out_path = os.path.join(PROJECT_ROOT, out) if not os.path.isabs(out) else out
    os.makedirs(os.path.dirname(out_path) or '.', exist_ok=True)
    with open(out_path, 'w', encoding='utf-8') as f:
        f.write(out_json)
    result = {
        'schema_file': out,
        'raw_bytes': raw_bytes,
        'schema_bytes': len(out_json),
        'compression': f'{raw_bytes / max(len(out_json), 1):.1f}x',
        # node_count 供编排器做「复杂度分档」（单图/极简组件合并 static+rewrite 为一个 subagent）
        'node_count': node_count,
    }
    # 默认只回计数（省主 agent 上下文）：这些明细的消费者是静态稿 subagent（读 schema.json
    # 内联的 renderer/componentId/layoutHint），主 agent 不据此行动，故默认不回逐条 id 列表。
    # --verbose 恢复全量明细（调试/人工核对时用）。
    if verbose:
        if inlined_instances:
            result['inlinedInstances'] = inlined_instances
            result['inlinedInstances_hint'] = (
                '以上组件实例（componentId + type:block）已内联/递归展开真实内部结构成普通 schema。'
                '静态稿照抄 style 还原、勿回母版/重拉/整块切；改写阶段按其文本/内容子节点绑数据，'
                '勿删掉重写；componentId 供判定是否复用项目现成组件。'
            )
        if wrapped_splits:
            result['wrappedTextSplits'] = wrapped_splits
            result['wrappedTextSplits_hint'] = (
                '以上纵向列已标 layoutHint=wrapped-text-split：wrappedTextGroup 列出的多个文本节点'
                '其实是同一段被设计师拆成多图层的绕排文字（工具做不出文字绕排 inline 徽章）。'
                '静态稿应把该组文本合并成一个文本渲染（徽章作其 inline 首子元素 + line-clamp 绕排），'
                '改写阶段按一个数据字段绑定，勿当「标题+副标题」两段分别绑定。'
            )
    else:
        # 计数摘要（默认）：只回数量 + 一行提示，明细已内联进 schema.json 供静态稿 subagent 消费
        counts = {}
        if inlined_instances:
            counts['inlinedInstances'] = len(inlined_instances)
        if wrapped_splits:
            counts['wrappedTextSplits'] = len(wrapped_splits)
        if counts:
            result['signals'] = counts
            result['signals_hint'] = (
                '信号明细（renderer/componentId/layoutHint/wrappedTextGroup）已内联进 schema.json，'
                '由静态稿 subagent 读取；主 agent 无需逐条明细。需要人工核对时加 --verbose 重跑。'
            )
    return result, out_json


def cmd_normalize(args):
    if args.raw:
        raw_text = read_file(args.raw)
    else:
        raw_text = sys.stdin.read().strip()
    if not raw_text:
        print("ERROR: 未提供原始 layerData（--raw <path> 或 stdin）", file=sys.stderr)
        sys.exit(1)

    try:
        data = json.loads(raw_text)
    except json.JSONDecodeError as e:
        print(f"ERROR: 原始 layerData JSON 解析失败: {e}", file=sys.stderr)
        sys.exit(1)

    # 剥外壳（MCP [{type:text,text}] / {data:{}} / 裸节点）
    data = _unwrap_layerdata(data)

    # 母版映射（编排器预取，--masters 可重复）：{母版根 id(=componentId): 母版根节点}，供 box 补容器默认
    masters = {}
    for mpath in (args.masters or []):
        try:
            m = json.loads(read_file(mpath))
        except json.JSONDecodeError as e:
            print(f"ERROR: 母版 {mpath} JSON 解析失败: {e}", file=sys.stderr)
            sys.exit(1)
        m = _unwrap_layerdata(m)
        if not (isinstance(m, dict) and m.get('id')):
            # 显式传了却剥不出节点 = 大概率喂了未识别外壳/错文件；静默跳过会让 box 容器默认丢失、
            # schema 退化且无报错——loud fail 逼调用方修，别静默降级。
            print(f"ERROR: 母版 {mpath} 未解析出有效节点（需含 id 的节点；"
                  f"若是 MCP 落盘的 [{{type:text,text}}] 外壳，本版已支持剥壳，请检查文件内容是否完整）",
                  file=sys.stderr)
            sys.exit(1)
        masters[m['id']] = m

    # 带 children 预取的母版（--masters-full 可重复）：供深层嵌套实例子孙按 refId 回填几何。
    # 仅在 list-masters 报 needsFullMaster（有 hug 解释不了的退化内容节点）时才拉、才传，
    # 一层实例/普通场景不给此参数、走原路径零变化。
    full_masters = []
    for mpath in (getattr(args, 'masters_full', None) or []):
        try:
            m = json.loads(read_file(mpath))
        except json.JSONDecodeError as e:
            print(f"ERROR: 全量母版 {mpath} JSON 解析失败: {e}", file=sys.stderr)
            sys.exit(1)
        m = _unwrap_layerdata(m)
        if not (isinstance(m, dict) and m.get('id')):
            print(f"ERROR: 全量母版 {mpath} 未解析出有效节点（需含 id 的节点；"
                  f"若是 MCP 落盘的 [{{type:text,text}}] 外壳，本版已支持剥壳，请检查文件内容是否完整）",
                  file=sys.stderr)
            sys.exit(1)
        full_masters.append(m)
    full_geom = build_full_geom(full_masters) if full_masters else None

    schema = normalize_node(data, 0, masters=masters or None, full_geom=full_geom)
    if schema is None:
        print("ERROR: 根节点被判为不可见/零尺寸，无法规范化", file=sys.stderr)
        sys.exit(1)

    result, out_json = _finalize_schema(schema, len(raw_text), args.out, args.verbose)
    if result is None:
        print(out_json)
    else:
        print(json.dumps(result, ensure_ascii=False, indent=2))


def node_has_visual(node: dict) -> bool:
    """节点是否自带视觉（背景色/背景图/圆角/padding/border 任一，非纯透明布局层）。
    供 verify-tree「带视觉容器须有 owner」校验判定。"""
    if first_visible_fill(node):  # 任意可见填充（纯色或图片）
        return True
    if border_radius(node) is not None:
        return True
    if border_style(node):
        return True
    if any(node.get(k) for k in ('paddingLeft', 'paddingRight', 'paddingTop', 'paddingBottom')):
        return True
    return False


def build_parentmap(node: dict, acc: dict):
    """遍历原始 layerData（不裁剪），累积扁平 nodeId → {parent, name, type, hasVisual}。"""
    if not isinstance(node, dict):
        return
    nid = node.get('id')
    if nid:
        acc[nid] = {
            'parent': node.get('parentId'),
            'name': node.get('name') or '',
            'type': node.get('type'),
            'hasVisual': node_has_visual(node),
        }
    for c in node.get('children') or []:
        build_parentmap(c, acc)


def _scan_masters(data: dict):
    """扫 root layerData → (boxMasters, needsFullMaster)（均为排序后 componentId 列表）。

    纯本地、不发 MCP。cmd_list_masters 与 build 共用（避免两套扫描逻辑漂移）。
    - boxMasters：box/expand 实例的母版 componentId，须逐个 get_node_data(childless) 预取补容器默认。
    - needsFullMaster：有「退化 + hug 解释不了」子孙（image/叶子缺几何）的实例，须带 children 拉母版子树按 refId 回填。
    """
    box_masters = set()

    def scan(n):
        if not isinstance(n, dict) or n.get('visible') is False:
            return
        _, _, atomic, expand_kind = classify(n)
        if expand_kind in ('box', 'expand') and n.get('componentId'):
            box_masters.add(n['componentId'])
        if not atomic:  # cut 实例 atomic=True，不下钻其内部；box/expand atomic=False，继续递归
            for c in n.get('children') or []:
                scan(c)

    scan(data)

    # 结构信号检测：哪些 componentId 实例有「退化 + hug 解释不了」的子孙 → 需带 children 补拉几何。
    # 判据（见 ui2code NESTED-INSTANCE-GEOMETRY-BACKFILL 提案 §2）：某节点几何缺（width 或 height
    # 为 None、非 text）且 hug 解释不了——即它是 image/叶子（无子内容可 hug）。纯 flex hug 容器
    # （block + 有子节点、靠子内容抱高/宽）虽也缺几何但合法、不触发。用结构信号而非聚合百分比
    # （百分比会把合法 hug 容器算进分子、虚高误导）。
    needs_full = set()

    def scan_full(n, owner_cid):
        if not isinstance(n, dict) or n.get('visible') is False:
            return
        type_out, _, atomic, _ = classify(n)
        if owner_cid:  # 归属某实例（其几何应由该实例母版子树提供）
            geom_missing = (n.get('width') is None or n.get('height') is None)
            if geom_missing and (n.get('type') or '').lower() != 'text':
                if type_out == 'image' or not n.get('children'):  # image / 叶子：hug 解释不了 → 触发
                    needs_full.add(owner_cid)
        if atomic:  # 整体单元（cut/图标组/图片）不下钻
            return
        next_owner = n.get('componentId') or owner_cid
        for c in n.get('children') or []:
            scan_full(c, next_owner)

    scan_full(data, None)
    return sorted(box_masters), sorted(needs_full)


def cmd_list_masters(args):
    """扫 root layerData，列出 box 实例需预取的唯一母版 componentId，供编排器补拉。

    纯本地：不发 MCP。输出给编排器——逐个 get_node_data 预取母版后 `normalize --masters` 喂入；
    未预取的 box 实例在 normalize 阶段自动降级为切图占位（软失败、像素安全）。
    """
    if args.raw:
        raw_text = read_file(args.raw)
    else:
        raw_text = sys.stdin.read().strip()
    if not raw_text:
        print("ERROR: 未提供原始 layerData（--raw <path> 或 stdin）", file=sys.stderr)
        sys.exit(1)
    try:
        data = json.loads(raw_text)
    except json.JSONDecodeError as e:
        print(f"ERROR: 原始 layerData JSON 解析失败: {e}", file=sys.stderr)
        sys.exit(1)
    if isinstance(data, dict) and 'data' in data and isinstance(data['data'], dict) and 'id' in data['data']:
        data = data['data']

    box_masters, needs_full = _scan_masters(data)

    print(json.dumps({
        'boxMasters': box_masters,
        'needsFullMaster': needs_full,
        'hint': ('对 boxMasters 逐个 get_node_data(designId, <id>, includeChildrenData=false) 预取母版，'
                 '再 normalize --masters <各母版落盘路径> 喂入；未预取的 box 实例会降级为切图占位，'
                 'expand 实例（含内容+套实例，如动态券卡）不降级、只是缺容器默认。'),
        'hint_full': ('needsFullMaster 非空：这些 componentId 实例有 hug 解释不了的退化子孙（image/叶子缺几何），'
                      '深层嵌套子孙的几何在母版子树里——须对它们 get_node_data(designId, <id>, '
                      'includeChildrenData=true) 带 children 拉取、落盘，再 normalize --masters-full <各落盘路径> '
                      '喂入，按 refId 回填子孙几何（否则 rect 退化 0,0,0,0、静态稿被迫估算坐标 → 金币错位/图标裁切）。'
                      '为空则无需——普通场景/一层实例不触发，守 childless 省 token 初衷。'),
        'note_childless': ('boxMasters 务必 includeChildrenData=false：normalize 只取母版根的容器默认'
                           '（padding/圆角/layout/尺寸/对齐，见 MASTER_INHERIT_PROPS），从不使用母版 children。'
                           'expand 实例的内部结构来自实例本次 layerData（非母版），故母版仍只需 childless。'
                           '拉子树纯属浪费——母版若是状态栏/导航等宿主件，其 children（图标 vector path）体量极大。'
                           '（needsFullMaster 是例外：退化时才对特定 componentId 带 children 拉取。）'),
    }, ensure_ascii=False, indent=2))


def cmd_parentmap(args):
    """从一份或多份 get_node_data 原始 layerData 生成整页扁平 parentId 映射。

    与 normalize 的区别：扁平（非嵌套）、不裁剪（全节点在册）、可合并多帧（整页级）。
    用途：组件树的**唯一拓扑真源**——`tree` 子命令据此 join 对照表语义生成 §2.0 组件树，
    verify-tree 据此校验对照表 nodeId 合法性与「带视觉容器」owner 归属。
    """
    raws = args.raw or []
    if not raws:
        print("ERROR: 未提供原始 layerData（--raw <path> 可重复）", file=sys.stderr)
        sys.exit(1)
    acc = {}
    for path in raws:
        raw_text = read_file(path)
        try:
            data = json.loads(raw_text)
        except json.JSONDecodeError as e:
            print(f"ERROR: {path} JSON 解析失败: {e}", file=sys.stderr)
            sys.exit(1)
        # 剥外壳（MCP [{type:text,text}] / {data:{}} / 裸节点）
        data = _unwrap_layerdata(data)
        build_parentmap(data, acc)

    out_json = json.dumps(acc, ensure_ascii=False, indent=2)
    if args.out:
        out_path = os.path.join(PROJECT_ROOT, args.out) if not os.path.isabs(args.out) else args.out
        os.makedirs(os.path.dirname(out_path) or '.', exist_ok=True)
        with open(out_path, 'w', encoding='utf-8') as f:
            f.write(out_json)
        print(json.dumps({
            'parentmap_file': args.out,
            'node_count': len(acc),
            'visual_containers': sum(1 for v in acc.values() if v['hasVisual']),
        }, ensure_ascii=False, indent=2))
    else:
        print(out_json)


# ---- tree 子命令：parentmap(拓扑真源) + §2.1 对照表(语义) → join 生成 §2.0 组件树 ----
# 翻转后组件树不再手搭：对照表提供节点集与语义标签，parentmap 提供父子拓扑，
# 脚本按「最近的、也在对照表里的祖先」定树父——拓扑正确 by construction。
# 解析/遍历公共件在 tree_util（与 verify-tree 共用）；本文件只留 relay 侧渲染与命令入口。

def _render_tree(nodes: list, children_map: dict) -> str:
    """DFS 渲染 ASCII 树，4 列缩进（├──/└──/│　＋　`    `），根为 Page。

    节点标签富化：`名称 (类型) nodeId:X [reuse:...] — 备注`——reuse/备注 来自对照表语义列，
    使 §2.0 组件树自带 结构 + 复用 + 坑点，成为唯一人审视图（§2.1 对照表降级为机器锚点表）。
    """
    def _label(nd: dict) -> str:
        s = f"{nd['name']} ({'骨架容器' if nd['skeleton'] else '业务组件'}) nodeId:{nd['nodeId']}"
        if nd.get('reuse'):
            s += f" [reuse:{nd['reuse']}]"
        if nd.get('note'):
            s += f" — {nd['note']}"
        return s

    label = {nd['nodeId']: _label(nd) for nd in nodes}
    out = ['Page']

    def walk(nid, prefix):
        kids = children_map.get(nid, [])
        for k, cid in enumerate(kids):
            last = k == len(kids) - 1
            out.append(f"{prefix}{'└── ' if last else '├── '}{label.get(cid, cid)}")
            walk(cid, prefix + ('    ' if last else '│   '))

    walk(None, '')
    return '\n'.join(out)


def cmd_tree(args):
    pm = json.loads(read_file(args.parentmap))
    nodes = parse_component_table(read_file(args.table))
    if not nodes:
        print("ERROR: --table 里未找到「层级 + nodeId」列的对照表", file=sys.stderr)
        sys.exit(1)
    node_set = {nd['nodeId'] for nd in nodes}
    children_map = {}
    for nd in nodes:
        parent = nearest_ancestor_in_set(nd['nodeId'], node_set, pm)
        children_map.setdefault(parent, []).append(nd['nodeId'])
    block = f"### 2.0 组件树\n\n```\n{_render_tree(nodes, children_map)}\n```"
    if args.out:
        out_path = os.path.join(PROJECT_ROOT, args.out) if not os.path.isabs(args.out) else args.out
        os.makedirs(os.path.dirname(out_path) or '.', exist_ok=True)
        with open(out_path, 'w', encoding='utf-8') as f:
            f.write(block + '\n')
        print(json.dumps({'tree_file': args.out, 'node_count': len(nodes)}, ensure_ascii=False, indent=2))
    else:
        print(block)


def _emit_build_fallback(reason: str, design_id: str, node_id: str):
    """build 直连 MCP 失败时的软回退：打印手动 prepare/normalize 流程 + 原因，退非零（2）。

    agent 读到后用自己的 MCP 会话按 manual_steps 逐步执行；鉴权失败须先按 relay-mcp.md
    刷新 token。build 只是把手动链收成一条命令的优化，直连不可用时不阻断、退回手动即可。
    """
    print(json.dumps({
        'status': 'fallback',
        'reason': reason,
        'designId': design_id,
        'nodeId': node_id,
        'manual_steps': [
            f'[required-tool] get_node_data(designId={design_id}, nodeId={node_id}, includeChildrenData=true) → 落盘 <raw>',
            'python3 .workflow/node/workflow-implement/relay-schema-gen.py list-masters --raw <raw>',
            '[required-tool] 对 boxMasters 逐个 get_node_data(includeChildrenData=false) 落盘 master_<id>.json；'
            'needsFullMaster 逐个 get_node_data(includeChildrenData=true) 落盘 master_full_<id>.json',
            'python3 .workflow/node/workflow-implement/relay-schema-gen.py normalize '
            '--raw <raw> --masters <各 master_<id>.json> --masters-full <各 master_full_<id>.json> --out <schema.json>',
        ],
        'hint': 'build 直连 relay MCP 失败（见 reason）。鉴权失败先按 relay-mcp.md 跑 '
                '`lbcli relay get-token` 刷新后重跑 build；其余按 manual_steps 用 agent 的 MCP 逐步执行。',
    }, ensure_ascii=False, indent=2))
    sys.exit(2)


def cmd_build(args):
    """一步取数（编排层内聚）：直连 relay MCP 跑完 get_node_data(root) → list-masters →
    母版预取 → normalize，产出精简 schema + 摘要。MCP 不可达/鉴权失败即软回退（见 _emit_build_fallback）。

    与分步 prepare/normalize 等价，只是把 agent 逐个手调 MCP 的往复收成一条命令。
    """
    context_content = read_file(f'delivery/{args.task}/context.md')
    design_id = extract_design_id(extract_design_url(context_content))
    node_id = args.node_id

    # 1. 取 root layerData（失败即软回退，不阻断）
    try:
        client = getattr(args, '_client', None) or _make_relay_client()
        root_content = fetch_node_data(client, design_id, node_id, True)
    except RelayMcpError as e:
        _emit_build_fallback(str(e), design_id, node_id)
        return

    root = _unwrap_layerdata(root_content)
    if not (isinstance(root, dict) and root.get('id')):
        print("ERROR: get_node_data 未解析出有效根节点（含 id）", file=sys.stderr)
        sys.exit(1)

    # 可选落盘 root raw（供调试/追溯，不进上下文）
    if not args.no_keep_raw:
        safe = re.sub(r'[^\w.-]', '_', node_id)
        raw_path = os.path.join(PROJECT_ROOT, f'delivery/{args.task}/workspace/relay-raw/{safe}.json')
        os.makedirs(os.path.dirname(raw_path), exist_ok=True)
        with open(raw_path, 'w', encoding='utf-8') as f:
            f.write(json.dumps(root_content, ensure_ascii=False))

    # 2. 列母版 + 3. 预取（母版拉失败不阻断：对应 box 在 normalize 自动降级切图）
    box_masters, needs_full = _scan_masters(root)
    masters = {}
    master_failed = []
    for cid in box_masters:
        try:
            m = _unwrap_layerdata(fetch_node_data(client, design_id, cid, False))
            if isinstance(m, dict) and m.get('id'):
                masters[m['id']] = m
            else:
                master_failed.append(cid)
        except RelayMcpError:
            master_failed.append(cid)
    full_masters = []
    for cid in needs_full:
        try:
            mf = _unwrap_layerdata(fetch_node_data(client, design_id, cid, True))
            if isinstance(mf, dict) and mf.get('id'):
                full_masters.append(mf)
        except RelayMcpError:
            pass  # 几何补拉失败：对应实例退化为估算，软失败不阻断
    full_geom = build_full_geom(full_masters) if full_masters else None

    # 4. normalize
    schema = normalize_node(root, 0, masters=masters or None, full_geom=full_geom)
    if schema is None:
        print("ERROR: 根节点被判为不可见/零尺寸，无法规范化", file=sys.stderr)
        sys.exit(1)

    result, _ = _finalize_schema(schema, len(json.dumps(root_content)), args.out, args.verbose)
    result['fetched'] = {'boxMasters': len(masters), 'needsFullMaster': len(full_masters)}
    if master_failed:
        result['fetched']['boxMasters_failed'] = master_failed
        result['fetched']['boxMasters_failed_hint'] = \
            '这些母版拉取失败，对应 box 实例已降级为切图占位（像素安全，不阻断）。'
    print(json.dumps(result, ensure_ascii=False, indent=2))


def main():
    parser = argparse.ArgumentParser(
        description='relay layerData → 精简 schema 规范化工具（替换 deco-ui-gen.py）'
    )
    subparsers = parser.add_subparsers(dest='command', required=True)

    p_prepare = subparsers.add_parser('prepare', help='构建 get_node_data 调用参数')
    p_prepare.add_argument('--task', required=True, help='任务名（用于定位 context.md）')
    p_prepare.add_argument('--node-id', required=True, help='Relay 节点 ID，如 45:21')

    p_norm = subparsers.add_parser('normalize', help='原始 layerData → 精简 schema')
    p_norm.add_argument('--raw', help='get_node_data 原始返回落盘路径（也可通过 stdin 传入）')
    p_norm.add_argument('--out', help='精简 schema 输出路径（缺省打印到 stdout）')
    p_norm.add_argument('--masters', action='append',
                        help='母版 JSON 落盘路径（可重复，按根 id=componentId 索引），供 box 实例补容器默认；'
                             '未提供则对应 box 降级为切图占位')
    p_norm.add_argument('--masters-full', action='append',
                        help='带 children 预取的母版 JSON 落盘路径（可重复）——供深层嵌套实例子孙按 refId 回填几何；'
                             '仅在 list-masters 报 needsFullMaster 时才需，普通场景不给')
    p_norm.add_argument('--verbose', action='store_true',
                        help='回全量信号明细（inlinedInstances/wrappedTextSplits 逐条 id）；'
                             '默认只回计数 + node_count，省主 agent 上下文（明细已内联进 schema.json）')

    p_lm = subparsers.add_parser('list-masters', help='列出 box 实例需预取的唯一母版 componentId')
    p_lm.add_argument('--raw', help='get_node_data 原始返回落盘路径（也可通过 stdin 传入）')

    p_pm = subparsers.add_parser('parentmap', help='原始 layerData → 整页扁平 parentId 映射（拓扑真源）')
    p_pm.add_argument('--raw', action='append', help='get_node_data 原始落盘路径，可重复传多帧合并')
    p_pm.add_argument('--out', help='parentmap 输出路径（建议 delivery/<task>/schema/parentmap.json）')

    p_tree = subparsers.add_parser('tree', help='parentmap + 对照表 → 生成 §2.0 组件树（翻转后不手搭）')
    p_tree.add_argument('--parentmap', required=True, help='parentmap.json 路径（拓扑真源）')
    p_tree.add_argument('--table', required=True,
                        help='对照表路径（语义源，delivery/<task>/schema/component-table.md；'
                             '对照表仍内联的老稿可传 design.md）')
    p_tree.add_argument('--out', help='组件树输出路径（缺省打印 stdout；建议直接写入 design.md §2.0）')

    p_build = subparsers.add_parser(
        'build', help='一步到位：直连 relay MCP 取数 + normalize（编排层内聚，推荐）')
    p_build.add_argument('--task', required=True, help='任务名（用于定位 context.md 取 designId）')
    p_build.add_argument('--node-id', required=True, help='组件根节点 ID，如 96:290')
    p_build.add_argument('--out', required=True, help='精简 schema 输出路径')
    p_build.add_argument('--verbose', action='store_true',
                         help='回全量信号明细（同 normalize --verbose）')
    p_build.add_argument('--no-keep-raw', action='store_true',
                         help='不落盘 root layerData（默认落 delivery/<task>/workspace/relay-raw/ 供追溯）')

    args = parser.parse_args()
    if args.command == 'prepare':
        cmd_prepare(args)
    elif args.command == 'build':
        cmd_build(args)
    elif args.command == 'normalize':
        cmd_normalize(args)
    elif args.command == 'list-masters':
        cmd_list_masters(args)
    elif args.command == 'parentmap':
        cmd_parentmap(args)
    elif args.command == 'tree':
        cmd_tree(args)


if __name__ == '__main__':
    main()
