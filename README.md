# WordWing

WordWing 是一个智能文本翻译工具，可以自动检测选中的文本并实时翻译成目标语言。目前支持中英文互译。

## 功能特性

- 🔄 自动检测选中文本
- 🌍 中英文互译（中文↔英文）
- 💬 弹窗显示翻译结果
- 🖱️ 翻译窗口显示在鼠标附近
- 📋 基于 X11 剪贴板的文本监控
- ✏️ 原地替换被翻译的文本（点击"替换"按钮）

## TODO
- [] 添加更多语言支持

- [X] 复制翻译文本
- [X] 触发翻译的快捷命令
- [X] 原地替换被翻译的文本

- [] 单词收藏
- [] 单词训练
## 系统要求

- Linux 系统（支持 X11）
- Rust 开发环境
- GTK3 开发库
- 网络连接（用于调用翻译 API）
- xclip（用于剪贴板操作）
- xdotool（用于文本替换功能，可选但推荐安装）

## 安装依赖

###  Fedora/CentOS/RHEL 系统
### 安装 Rust
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
### 安装系统依赖
```
sudo dnf install gtk3-devel pango-devel atk-devel cairo-devel gdk-pixbuf2-devel glib2-devel openssl-devel xclip xdotool
```

注意：`xdotool` 用于文本替换功能。如果未安装，替换功能将回退到使用剪贴板方式，但可能在某些应用中无法正常工作。
### 构建和运行
#### 克隆项目
```
git clone <repository-url>
cd WordWing
```
#### 构建项目
```
cargo build --release
```

#### 运行程序
```
cargo run
```
或者直接运行编译后的二进制文件：
```
./target/release/WordWing
```

## 桌面集成
### 创建桌面启动器
#### 创建 WordWing.desktop 文件：
```
ini
[Desktop Entry]
Version=1.0
Type=Application
Name=WordWing Translator
Comment=Translate selected text automatically
Exec=sh -c 'cd /local/bin/WordWing && DASHSCOPE_API_KEY={DASHSCOPE_API_KEY} cargo run'
Icon=accessories-dictionary
Terminal=false
Categories=Utility;TextTools;
Keywords=translation;clipboard;text;chinese;english;
```
#### 安装桌面启动器
```
# 复制 desktop 文件到系统目录
sudo cp WordWing.desktop /usr/share/applications/
# 或者复制到用户目录
cp WordWing.desktop ~/.local/share/applications/
更新图标缓存（可选）
bash
# 更新系统图标缓存
sudo gtk-update-icon-cache /usr/share/icons/hicolor
# 或者更新用户图标缓存
gtk-update-icon-cache ~/.local/share/icons
```
#### 配置
```
export DASHSCOPE_API_KEY="YOUR_API_KEY"
```

## 故障排除
### 常见问题
1. 无法检测选中文本

- 确保在 X11 环境下运行
- 检查是否有其他剪贴板管理器冲突
- 确认程序有访问 X11 的权限
2.  弹窗不显示

- 确保 GTK 环境正常
- 检查是否有足够的权限显示窗口
- 查看日志输出以获取更多信息
3. 编译错误

- 确保所有系统依赖已正确安装
- 检查 PKG_CONFIG_PATH 环境变量设置
- 运行 pkg-config 命令验证库文件是否存在
### 调试
启用详细日志输出：

```
RUST_LOG=debug cargo run
```
## 技术架构
- 语言: Rust
-异步运行时: Tokio
- GUI 框架: GTK3
- 剪贴板监控: x11-clipboard
- HTTP 客户端: reqwest
- 翻译服务: 阿里云 DashScope API

## 项目结构
```
.
├── src/
│   ├── main.rs              # 主程序入口
│   ├── selection_monitor.rs # 文本选中监控模块
│   ├── translator.rs        # 翻译 API 接口模块
│   └── popup_window.rs      # 弹窗显示模块
├── Cargo.toml               # 项目依赖配置
├── README.md                # 项目说明文档
└── WordWing.desktop         # 桌面启动器文件
```
## 开发
### VS Code 配置

项目包含 VS Code 调试配置：

- .vscode/launch.json - 调试配置
- .vscode/tasks.json - 构建任务

### 依赖库说明
- gtk 和 gdk - 图形界面库
- tokio - 异步运行时
- reqwest - HTTP 客户端
- x11-clipboard - X11 剪贴板访问
- tracing 和 tracing-subscriber - 日志系统
- x11rb - X11 协议实现

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request 来改进这个项目。
