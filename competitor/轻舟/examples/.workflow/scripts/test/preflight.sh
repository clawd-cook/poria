#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# E2E Preflight：环境选择 → 前置检查 → 跑测
#
# 用法：
#   bash .workflow/scripts/test/preflight.sh [test|local] [--task <task-name>] [-- <playwright-args>]
#
# 示例：
#   bash .workflow/scripts/test/preflight.sh test --task login-flow
#   bash .workflow/scripts/test/preflight.sh local --task login-flow -- --headed
#   bash .workflow/scripts/test/preflight.sh test  # 跑全量
# ─────────────────────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# ── 颜色 ──
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${CYAN}[preflight]${NC} $1"; }
ok()    { echo -e "${GREEN}[✓]${NC} $1"; }
warn()  { echo -e "${YELLOW}[!]${NC} $1"; }
fail()  { echo -e "${RED}[✗]${NC} $1"; exit 1; }

# ── 参数解析 ──
MODE=""
TASK=""
PW_ARGS=()
PARSING_PW=false

for arg in "$@"; do
  if $PARSING_PW; then
    PW_ARGS+=("$arg")
    continue
  fi
  case "$arg" in
    test|local)
      MODE="$arg" ;;
    --task)
      shift_next=true ;;
    --)
      PARSING_PW=true ;;
    *)
      if [[ "${shift_next:-}" == "true" ]]; then
        TASK="$arg"
        shift_next=false
      fi
      ;;
  esac
done

# ── 模式选择（未指定时交互询问）──
if [[ -z "$MODE" ]]; then
  echo ""
  echo "┌──────────────────────────────────────────────┐"
  echo "│     E2E 测试环境选择                          │"
  echo "├──────────────────────────────────────────────┤"
  echo "│  1) test   — 直连测试环境（页面已部署）        │"
  echo "│  2) local  — 本地开发（whistle 代理到本地）    │"
  echo "└──────────────────────────────────────────────┘"
  echo ""
  read -rp "选择模式 [1/2，默认 1]: " choice
  case "$choice" in
    2|local) MODE="local" ;;
    *) MODE="test" ;;
  esac
fi

info "测试模式: $MODE"
echo ""

# ── 环境配置 ──
if [[ -n "${TEST_BASE_URL:-}" ]]; then
  info "TEST_BASE_URL 已设置: $TEST_BASE_URL"
else
  read -rp "$(echo -e "${CYAN}[preflight]${NC}") TEST_BASE_URL: " input_url
  if [[ -z "$input_url" ]]; then
    fail "TEST_BASE_URL 不能为空，请输入被测环境的 URL"
  fi
  export TEST_BASE_URL="$input_url"
fi

case "$MODE" in
  test)
    unset TEST_PROXY 2>/dev/null || true
    ;;
  local)
    if [[ -n "${TEST_PROXY:-}" ]]; then
      info "TEST_PROXY 已设置: $TEST_PROXY"
    else
      DEFAULT_PROXY="http://127.0.0.1:12345"
      read -rp "$(echo -e "${CYAN}[preflight]${NC}") TEST_PROXY [默认 $DEFAULT_PROXY]: " input_proxy
      export TEST_PROXY="${input_proxy:-$DEFAULT_PROXY}"
    fi
    ;;
esac

# ════════════════════════════════════════════════════════════════════════════════
# 前置检查
# ════════════════════════════════════════════════════════════════════════════════

info "开始前置检查..."
echo ""

# ── 1. Playwright 浏览器 ──
if npx playwright install --dry-run chromium >/dev/null 2>&1 || command -v chromium >/dev/null 2>&1; then
  ok "Playwright chromium 已安装"
else
  warn "Playwright chromium 未安装，正在安装..."
  npx playwright install chromium
  ok "Playwright chromium 安装完成"
fi

# ── 2. Cookie / 登录态 ──
if [[ -n "${TEST_COOKIE:-}" ]]; then
  ok "TEST_COOKIE 已设置（环境变量）"
else
  info "TEST_COOKIE 未设置，尝试 lbcli 自动获取..."
  if command -v lbcli >/dev/null 2>&1; then
    COOKIE_RESULT=$(lbcli getssologin getssologin -f plain 2>/dev/null || echo "")
    if [[ -n "$COOKIE_RESULT" ]]; then
      export TEST_COOKIE="$COOKIE_RESULT"
      ok "通过 lbcli 获取 Cookie 成功"
    else
      fail "lbcli 获取 Cookie 失败。请手动设置：\n  export TEST_COOKIE=\"your_cookie_here\""
    fi
  else
    fail "lbcli 未安装且 TEST_COOKIE 未设置。请手动设置：\n  export TEST_COOKIE=\"your_cookie_here\""
  fi
fi

# ── 3. 模式相关检查 ──
case "$MODE" in
  test)
    info "检查测试环境可达性..."
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 "$TEST_BASE_URL" 2>/dev/null || echo "000")
    if [[ "$HTTP_CODE" == "000" ]]; then
      fail "无法连接 $TEST_BASE_URL（网络不通或 VPN 未连接）"
    elif [[ "$HTTP_CODE" =~ ^(5[0-9]{2})$ ]]; then
      fail "$TEST_BASE_URL 返回 $HTTP_CODE（服务端错误，确认测试环境是否正常）"
    else
      ok "测试环境可达 (HTTP $HTTP_CODE)"
    fi
    ;;
  local)
    # 检查 whistle
    if pgrep -f "w2" >/dev/null 2>&1 || pgrep -f "whistle" >/dev/null 2>&1; then
      ok "whistle 代理已运行"
    else
      warn "whistle 未运行，尝试启动..."
      if command -v w2 >/dev/null 2>&1; then
        w2 start -D >/dev/null 2>&1
        sleep 2
        if pgrep -f "w2" >/dev/null 2>&1; then
          ok "whistle 启动成功"
        else
          fail "whistle 启动失败，请手动执行 w2 start"
        fi
      else
        fail "whistle 未安装。请执行：npm install -g whistle && w2 start"
      fi
    fi

    # 检查本地 dev server
    info "检查本地 dev server (localhost:3000)..."
    LOCAL_CODE=$(curl -s -o /dev/null -w "%{http_code}" --max-time 5 "http://localhost:3000" 2>/dev/null || echo "000")
    if [[ "$LOCAL_CODE" == "000" ]]; then
      fail "本地 dev server 未启动。请先在项目根目录执行：npm run dev"
    else
      ok "本地 dev server 运行中 (HTTP $LOCAL_CODE)"
    fi
    ;;
esac

# ════════════════════════════════════════════════════════════════════════════════
# 前置检查通过，准备跑测
# ════════════════════════════════════════════════════════════════════════════════

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
info "前置检查全部通过"
echo ""
info "环境变量:"
echo "  TEST_BASE_URL = $TEST_BASE_URL"
echo "  TEST_PROXY    = ${TEST_PROXY:-（未设置）}"
echo "  TEST_COOKIE   = ${TEST_COOKIE:0:30}..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# ── 构建 playwright 命令 ──
CMD=(npx playwright test --config=playwright.config.ts)

if [[ -n "$TASK" ]]; then
  CMD+=("tasks/$TASK/")
  info "测试范围: tasks/$TASK/"
fi

CMD+=("${PW_ARGS[@]}")

info "执行: ${CMD[*]}"
echo ""

exec "${CMD[@]}"
