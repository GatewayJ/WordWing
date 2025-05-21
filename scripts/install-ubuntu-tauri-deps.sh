#!/usr/bin/env bash
# Ubuntu / Debian：安装 WordWing（Tauri 2）桌面端构建与运行所需的系统包。
# 用法：bash scripts/install-ubuntu-tauri-deps.sh
# 需管理员权限：sudo bash scripts/install-ubuntu-tauri-deps.sh
#
# 若 apt 因「v2rayA 源 EXPKEYSIG / 没有数字签名」失败，可先自动禁用该源再装依赖：
#   sudo WORDWING_FIX_V2RAYA=1 bash scripts/install-ubuntu-tauri-deps.sh
# 或自行编辑 /etc/apt/sources.list.d/ 下含 v2raya 的 .list，在 deb 行首加 #，再 sudo apt update。

set -euo pipefail

if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
  echo "请使用 sudo 运行，例如："
  echo "  sudo bash scripts/install-ubuntu-tauri-deps.sh"
  exit 1
fi

# 注释仍未以 # 开头的、包含 v2raya.org 的行（避免重复运行产生 ##）
fix_v2raya_sources() {
  local f
  for f in /etc/apt/sources.list.d/*.list; do
    [[ -f "$f" ]] || continue
    if grep -qi 'v2raya\.org' "$f" 2>/dev/null; then
      echo "备份并临时禁用 v2rayA 源: $f"
      cp -a "$f" "${f}.bak-wordwing-$(date +%Y%m%d%H%M%S)"
      sed -i '/v2raya\.org/I{/^[[:space:]]*#/!s/^/# /;}' "$f"
    fi
  done
}

export DEBIAN_FRONTEND=noninteractive

if [[ "${WORDWING_FIX_V2RAYA:-}" == "1" ]]; then
  echo "WORDWING_FIX_V2RAYA=1：处理可能过期的 v2rayA apt 源…"
  fix_v2raya_sources
fi

if ! apt-get update -qq; then
  echo ""
  echo "apt-get update 失败。若上方出现 v2raya / EXPKEYSIG / 没有数字签名，说明该第三方源密钥过期或无效。"
  echo "处理方式（二选一）："
  echo "  1) 自动注释 sources.list.d 中含 v2raya.org 的行后重跑本脚本："
  echo "       sudo WORDWING_FIX_V2RAYA=1 bash scripts/install-ubuntu-tauri-deps.sh"
  echo "  2) 手动：sudo nano /etc/apt/sources.list.d/（找到 v2raya 相关 .list），在 deb 行首加 #，再 sudo apt update"
  echo "  3) 按 v2rayA 官方文档更新 GPG 密钥后，再 sudo apt update"
  echo ""
  exit 1
fi

apt-get install -y \
  pkg-config \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libxdo-dev \
  libgtk-3-dev \
  xclip

echo ""
echo "系统依赖已安装（含 Tauri WebKitGTK 与 legacy-gtk 常用 GTK3 头文件）。"
echo "接下来在项目根目录执行："
echo "  npm install"
echo "  npm run tauri:dev"
echo ""
