.PHONY: help check build clean package package-all install uninstall

# Variables
VERSION := $(shell grep '^version' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

help: ## 显示帮助信息
	@echo "Lingbase 构建工具"
	@echo ""
	@echo "用法: make <目标>"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "本地安装:"
	@echo "  make install     # 直接运行"
	@echo "  make uninstall   # 停止进程"
	@echo ""
	@echo "部署到远端请使用脚本:"
	@echo "  ./scripts/remote-deploy.sh <服务器>    # 一键编译+打包+部署"
	@echo "  ./scripts/remote-deploy.sh --list     # 列出可用服务器"
	@echo "  ./scripts/deploy.sh <服务器>          # 仅部署已有包"
	@echo "  ./scripts/deploy.sh --uninstall      # 卸载"

check: ## 检查代码
	cargo check

build: ## 构建当前平台
	cargo build --release

build-x86_64: ## 构建 x86_64
	cargo build --release
	@mv -f target/release/lingbase target/release/lingbase-cpu 2>/dev/null || true

build-aarch64: ## 构建 ARM64（需要交叉编译工具链）
	@if ! command -v aarch64-linux-gnu-gcc > /dev/null 2>&1; then \
		echo "安装交叉编译工具链..."; \
		sudo apt-get install -y gcc-aarch64-linux-gnu || exit 1; \
	fi
	@rustup target add aarch64-unknown-linux-gnu 2>/dev/null || true
	CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc CARGO_TARGET=aarch64-unknown-linux-gnu cargo build --release --target aarch64-unknown-linux-gnu

build-x86_64-cuda: ## 构建 x86_64 CUDA 版
	cargo build --release --features cuda
	@mv -f target/release/lingbase target/release/lingbase-cuda 2>/dev/null || true

build-all: ## 构建所有架构版本
	make build-x86_64
	make build-x86_64-cuda
	-$(MAKE) build-aarch64

clean: ## 清理构建产物
	cargo clean
	rm -rf dist/

package: ## 打包当前平台
	make clean
	make build
	mkdir -p dist/lingbase-$(VERSION)-x86_64/lib
	mkdir -p dist/lingbase-$(VERSION)-x86_64/config
	cp target/release/lingbase dist/lingbase-$(VERSION)-x86_64/
	cp -r lib/x86_64/* dist/lingbase-$(VERSION)-x86_64/lib/ 2>/dev/null || true
	cp config/environment.toml dist/lingbase-$(VERSION)-x86_64/config/
	cp scripts/run.sh dist/lingbase-$(VERSION)-x86_64/
	tar -czf dist/lingbase-$(VERSION)-x86_64.tar.gz -C dist lingbase-$(VERSION)-x86_64
	@echo "打包完成: dist/lingbase-$(VERSION)-x86_64.tar.gz"
	@ls -lh dist/lingbase-$(VERSION)-x86_64.tar.gz

package-x86_64: ## 打包 x86_64
	make clean
	make build
	mkdir -p dist/lingbase-$(VERSION)-x86_64/lib
	mkdir -p dist/lingbase-$(VERSION)-x86_64/config
	cp target/release/lingbase dist/lingbase-$(VERSION)-x86_64/
	cp -r lib/x86_64/* dist/lingbase-$(VERSION)-x86_64/lib/
	cp config/environment.toml dist/lingbase-$(VERSION)-x86_64/config/
	cp scripts/run.sh dist/lingbase-$(VERSION)-x86_64/
	tar -czf dist/lingbase-$(VERSION)-x86_64.tar.gz -C dist lingbase-$(VERSION)-x86_64
	@echo "打包完成: dist/lingbase-$(VERSION)-x86_64.tar.gz"

package-x86_64-cuda: ## 打包 x86_64 CUDA 版
	make clean
	cargo build --release --features cuda
	mkdir -p dist/lingbase-$(VERSION)-x86_64-cuda/lib/cuda
	mkdir -p dist/lingbase-$(VERSION)-x86_64-cuda/config
	cp target/release/lingbase dist/lingbase-$(VERSION)-x86_64-cuda/
	cp -r lib/cuda/* dist/lingbase-$(VERSION)-x86_64-cuda/lib/cuda/
	cp config/environment.toml dist/lingbase-$(VERSION)-x86_64-cuda/config/
	cp scripts/run.sh dist/lingbase-$(VERSION)-x86_64-cuda/
	tar -czf dist/lingbase-$(VERSION)-x86_64-cuda.tar.gz -C dist lingbase-$(VERSION)-x86_64-cuda
	@echo "打包完成: dist/lingbase-$(VERSION)-x86_64-cuda.tar.gz"

package-aarch64: ## 打包 ARM64
	make clean
	make build-aarch64
	mkdir -p dist/lingbase-$(VERSION)-aarch64/lib/aarch64
	mkdir -p dist/lingbase-$(VERSION)-aarch64/config
	cp target/aarch64-unknown-linux-gnu/release/lingbase dist/lingbase-$(VERSION)-aarch64/
	cp lib/aarch64/*.so* dist/lingbase-$(VERSION)-aarch64/lib/aarch64/
	cp config/environment.toml dist/lingbase-$(VERSION)-aarch64/config/
	cp scripts/run.sh dist/lingbase-$(VERSION)-aarch64/
	tar -czf dist/lingbase-$(VERSION)-aarch64.tar.gz -C dist lingbase-$(VERSION)-aarch64
	@echo "打包完成: dist/lingbase-$(VERSION)-aarch64.tar.gz"

package-all: ## 打包所有架构版本
	make build-x86_64
	mkdir -p dist/lingbase-$(VERSION)-x86_64/lib
	mkdir -p dist/lingbase-$(VERSION)-x86_64/config
	cp target/release/lingbase-cpu dist/lingbase-$(VERSION)-x86_64/
	cp -r lib/x86_64/* dist/lingbase-$(VERSION)-x86_64/lib/
	cp config/environment.toml dist/lingbase-$(VERSION)-x86_64/config/
	cp scripts/run.sh dist/lingbase-$(VERSION)-x86_64/
	tar -czf dist/lingbase-$(VERSION)-x86_64.tar.gz -C dist lingbase-$(VERSION)-x86_64
	@echo "打包完成: dist/lingbase-$(VERSION)-x86_64.tar.gz"
	make build-x86_64-cuda
	mkdir -p dist/lingbase-$(VERSION)-x86_64-cuda/lib/cuda
	mkdir -p dist/lingbase-$(VERSION)-x86_64-cuda/config
	cp target/release/lingbase-cuda dist/lingbase-$(VERSION)-x86_64-cuda/
	cp -r lib/cuda/* dist/lingbase-$(VERSION)-x86_64-cuda/lib/cuda/
	cp config/environment.toml dist/lingbase-$(VERSION)-x86_64-cuda/config/
	cp scripts/run.sh dist/lingbase-$(VERSION)-x86_64-cuda/
	tar -czf dist/lingbase-$(VERSION)-x86_64-cuda.tar.gz -C dist lingbase-$(VERSION)-x86_64-cuda
	@echo "打包完成: dist/lingbase-$(VERSION)-x86_64-cuda.tar.gz"
	-$(MAKE) build-aarch64
	mkdir -p dist/lingbase-$(VERSION)-aarch64/lib
	mkdir -p dist/lingbase-$(VERSION)-aarch64/config
	cp target/aarch64-unknown-linux-gnu/release/lingbase-cpu dist/lingbase-$(VERSION)-aarch64/
	cp -r lib/aarch64/* dist/lingbase-$(VERSION)-aarch64/lib/ 2>/dev/null || true
	cp config/environment.toml dist/lingbase-$(VERSION)-aarch64/config/
	cp scripts/run.sh dist/lingbase-$(VERSION)-aarch64/
	tar -czf dist/lingbase-$(VERSION)-aarch64.tar.gz -C dist lingbase-$(VERSION)-aarch64
	@echo "打包完成: dist/lingbase-$(VERSION)-aarch64.tar.gz"
	@echo ""
	@echo "所有包:"
	@ls -lh dist/lingbase-*.tar.gz 2>/dev/null

install: ## 本地运行
	./target/release/lingbase

uninstall: ## 停止本地 lingbase 进程
	-pkill -f lingbase || true
	@echo "已停止 lingbase 进程"