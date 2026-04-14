# WordWing — 常用命令（需在仓库根目录执行）

.PHONY: help run pre-commit

help:
	@echo "make run         启动 Tauri 开发模式（npm run tauri:dev）"
	@echo "make pre-commit  Rust 格式检查 + clippy + test，前端 tsc --noEmit"
	@echo "./scripts/install-or-update.sh  构建并安装到 ~/.local/bin + systemd --user（见 packaging/systemd/）"

run:
	npm run tauri:dev

pre-commit:
	cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
	cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
	cargo test --manifest-path src-tauri/Cargo.toml
	npx tsc --noEmit
