#!/usr/bin/env bash
# 将 WordWing 安装为 systemd 用户单元（随图形会话管理，适合 Linux 桌面）。
# 与一键脚本 scripts/install-or-update.sh 使用同一单元模板 wordwing-user.service。
#
# 用法：
#   ./install-user-service.sh [wordwing 可执行文件的绝对或相对路径]
# 未传路径时依次尝试：PATH 中的 wordwing、本仓库 src-tauri/target/release/wordwing
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
UNIT_SRC="$SCRIPT_DIR/wordwing-user.service"
UNIT_DST="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/wordwing.service"

resolve_binary() {
  if [[ -n "${1:-}" ]]; then
    local p="$1"
    [[ -x "$p" ]] || p="$(command -v "$p" 2>/dev/null || true)"
    if [[ -n "$p" && -x "$p" ]]; then
      realpath "$p" 2>/dev/null || readlink -f "$p"
      return
    fi
    if [[ -f "$1" ]]; then
      realpath "$1" 2>/dev/null || readlink -f "$1"
      return
    fi
  fi
  if command -v wordwing &>/dev/null; then
    realpath "$(command -v wordwing)" 2>/dev/null || readlink -f "$(command -v wordwing)"
    return
  fi
  local rel="$REPO_ROOT/src-tauri/target/release/wordwing"
  if [[ -x "$rel" ]]; then
    realpath "$rel" 2>/dev/null || readlink -f "$rel"
    return
  fi
  echo "未找到可执行的 wordwing。请先 npm run tauri:build 或传入二进制路径。" >&2
  exit 1
}

BINARY="$(resolve_binary "${1:-}")"
mkdir -p "$(dirname "$UNIT_DST")"
BINARY="$BINARY" awk '
  /^ExecStart=/ { print "ExecStart=" ENVIRON["BINARY"]; next }
  { print }
' "$UNIT_SRC" >"$UNIT_DST"
echo "已写入: $UNIT_DST"
echo "ExecStart=$BINARY"
systemctl --user daemon-reload
systemctl --user enable --now wordwing.service
echo "已执行: systemctl --user enable --now wordwing.service"
echo "状态:   systemctl --user status wordwing.service"
echo "日志:   journalctl --user -u wordwing.service -f"
echo ""
echo "提示：若托盘/主窗口不显示，请在图形桌面终端执行后重试："
echo "  systemctl --user import-environment DISPLAY WAYLAND_DISPLAY XDG_SESSION_TYPE"
echo "  systemctl --user restart wordwing.service"
