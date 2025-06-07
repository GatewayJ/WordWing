# WordWing

个人桌面助手：**英语**（生词、复习、周短文）与 **Todo**（条目、定时），数据本地优先。产品形态与视觉以根目录 **`DESIGN.md`** 为准。

## 技术栈（当前）

| 部分 | 说明 |
|------|------|
| **主应用** | [Tauri 2](https://tauri.app/) + **React** + **TypeScript** + **Vite** |
| **路由** | `react-router-dom`，与 `DESIGN.md` 中 IA 一致：`/english/*`、`/todo/*`、`/settings` |
| **字体** | `@fontsource` 打包（Instrument Serif + DM Sans），符合「自托管、不依赖外网 CDN」方向 |
| **旧原型** | `legacy-gtk/` — 原 GTK + X11 + 全局热键翻译，已不再作为主开发线 |

## 划词翻译与生词（已实现）

- **全局快捷键**（默认 **Ctrl+Shift+1**，与数字行 **1 / !** 同键）：通过 **arboard** 读取 **PRIMARY**（划词）再读标准 **剪贴板**（Linux 上支持 X11 与 Wayland data-control）；调用 DashScope 翻译后，在 **`translate-overlay`** 浮层展示。可在应用内 **设置 → 翻译快捷键** 换预设并立即生效（配置写入 `app_settings.json`）。
- **浮层**：「收藏」写入本地 `vocabulary.json`（应用数据目录，与 bundle identifier 对应，如 `~/.local/share/com.wordwing.desktop/`）；「用剪贴板再试」「重试」已接命令。
- **生词页**：表格 **zebra** 行样式；监听 `vocabulary-changed`，与浮层收藏联动刷新；支持删除单行。

配置环境变量 **`DASHSCOPE_API_KEY`**（见设置页说明）。**不再依赖** 系统安装 **`xclip`**。

**Wayland 全局快捷键：** 除 `tauri-plugin-global-shortcut` 外，在检测到 **`WAYLAND_DISPLAY`** 时会通过 **xdg-desktop-portal GlobalShortcuts** 再注册一遍；**首次启动或绑定失败重试时**，系统可能弹出对话框要求授权，请确认。若门户不可用，仍可用生词页 **「打开翻译浮层（划词）」**。

## 环境要求

- **Node.js** ≥ 20.19（或 22.12+），以满足 Vite 7 的 engine 要求；略低版本可能仍能构建，但会收到警告。
- **Rust** stable、`cargo`（建议 [rustup](https://rustup.rs/)）
- **Linux 桌面：** 构建 Tauri 需系统依赖（WebKitGTK 等），参见 [Tauri Linux 前置条件](https://tauri.app/start/prerequisites/)。

### Ubuntu / Debian：一键安装系统依赖

在仓库根目录执行（需要 `sudo` 密码）：

```bash
sudo bash scripts/install-ubuntu-tauri-deps.sh
```

安装完成后即可 `npm run tauri:dev`。若构建仍报缺少 `.pc` 或 `webkit2gtk`，请确认已安装 `libwebkit2gtk-4.1-dev` 与 `pkg-config`。

若 `apt-get update` 因 **v2rayA 源**（`apt.v2raya.org`）报 **EXPKEYSIG / 没有数字签名**，与 WordWing 无关；可先禁用该源再装依赖：

```bash
sudo WORDWING_FIX_V2RAYA=1 bash scripts/install-ubuntu-tauri-deps.sh
```

或手动编辑 `/etc/apt/sources.list.d/` 下对应 `.list`，在 `deb` 行首加 `#`，执行 `sudo apt update` 后再运行安装脚本。

## 开发

```bash
npm install
npm run dev          # 仅前端（浏览器 http://localhost:1420）
```

在 **未设置 `CI=1`** 的环境下运行 Tauri（部分环境会把 `CI=1` 传给 CLI 导致参数错误）。仓库已提供脚本：

```bash
npm run tauri:dev
```

等价于 `env -u CI tauri dev`。构建桌面包：

```bash
npm run tauri:build
```

## 构建

```bash
npm run build        # 前端产物 → dist/
npm run tauri:build  # 完整桌面包（需系统 WebKit/GTK 依赖）
```

## 旧版 GTK 原型

```bash
cd legacy-gtk
cargo run
```

需 GTK3、X11、`DASHSCOPE_API_KEY` 等；详见旧文档与 `legacy-gtk/Cargo.toml`。

## 文档

- **`DESIGN.md`** — 设计系统、信息架构、组件与无障碍约定
- **`CLAUDE.md`** — 改 UI 前必读 `DESIGN.md`
- **`docs/plan-design-review.md`**、**`docs/plan-eng-review.md`** — 设计/工程审查记录

## 后续实现（路线图摘要）

- LevelDB、划词浮层、X11 `SelectionSource` 与 Tauri 命令层（见 `docs/plan-eng-review.md`）
- CI：`cargo fmt/clippy/test`、前端 `build`
