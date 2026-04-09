#!/usr/bin/env bash
# WordWing 一键安装或更新（Linux）
set -euo pipefail

usage() {
  cat <<'EOF'
WordWing 一键安装或更新（Linux）
  - npm install（可选）、tauri release 构建、安装到 ~/.local/bin/wordwing
  - 写入/更新 systemd 用户单元并重启服务

用法:
  ./scripts/install-or-update.sh
  ./scripts/install-or-update.sh --no-build          不编译，仅用已有 target/release/wordwing
  ./scripts/install-or-update.sh --no-npm-install    不跑 npm install，直接 tauri:build
  ./scripts/install-or-update.sh --bin-only          只拷贝二进制，不配置 systemd
EOF
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN_NAME="wordwing"
INSTALL_DIR="${HOME}/.local/bin"
INSTALLED="${INSTALL_DIR}/${BIN_NAME}"
BUILD_ARTIFACT="${REPO_ROOT}/src-tauri/target/release/${BIN_NAME}"
INSTALL_SERVICE="${REPO_ROOT}/packaging/systemd/install-user-service.sh"

NO_BUILD=0
NO_NPM_INSTALL=0
BIN_ONLY=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-build) NO_BUILD=1 ;;
    --no-npm-install) NO_NPM_INSTALL=1 ;;
    --bin-only) BIN_ONLY=1 ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "未知参数: $1（见 --help）" >&2
      exit 1
      ;;
  esac
  shift
done

cd "$REPO_ROOT"

if [[ "$NO_BUILD" -eq 0 ]]; then
  if [[ "$NO_NPM_INSTALL" -eq 0 ]]; then
    npm install
  fi
  npm run tauri:build
fi

if [[ ! -x "$BUILD_ARTIFACT" ]]; then
  echo "找不到可执行文件: $BUILD_ARTIFACT" >&2
  echo "请先执行构建，或去掉 --no-build。" >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
cp -f "$BUILD_ARTIFACT" "$INSTALLED"
chmod +x "$INSTALLED"
echo "已安装: $INSTALLED"

if [[ "$BIN_ONLY" -eq 1 ]]; then
  echo "已跳过 systemd（--bin-only）。若需用户服务，请执行:"
  echo "  $INSTALL_SERVICE $INSTALLED"
  exit 0
fi

if [[ ! -x "$INSTALL_SERVICE" ]]; then
  echo "缺少: $INSTALL_SERVICE" >&2
  exit 1
fi

"$INSTALL_SERVICE" "$INSTALLED"
# 覆盖安装后需重启才能加载磁盘上新二进制（同一路径时进程仍可能映射旧文件）
systemctl --user restart wordwing.service 2>/dev/null \
  || systemctl --user start wordwing.service 2>/dev/null \
  || true
echo "完成。状态: systemctl --user status wordwing.service"
