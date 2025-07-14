# Claude Code Rust Makefile

.PHONY: help build test clean install dev docker benchmark lint format check coverage docs release

# 默认目标
help: ## 显示帮助信息
	@echo "Claude Code Rust - 可用命令:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# 构建相关
build: ## 构建项目
	cargo build

build-release: ## 构建发布版本
	cargo build --release

build-all: ## 构建所有特性
	cargo build --all-features

# 测试相关
test: ## 运行所有测试
	cargo test

test-unit: ## 运行单元测试
	cargo test --lib

test-integration: ## 运行集成测试
	cargo test --test integration_tests

test-watch: ## 监控文件变化并运行测试
	cargo watch -x test

# 代码质量
lint: ## 运行 clippy 检查
	cargo clippy --all-targets --all-features -- -D warnings

format: ## 格式化代码
	cargo fmt --all

format-check: ## 检查代码格式
	cargo fmt --all -- --check

check: ## 快速检查代码
	cargo check --all-targets --all-features

audit: ## 安全审计
	cargo audit

# 基准测试
benchmark: ## 运行基准测试
	cargo bench

benchmark-save: ## 运行基准测试并保存结果
	cargo bench -- --save-baseline main

benchmark-compare: ## 比较基准测试结果
	cargo bench -- --baseline main

# 覆盖率
coverage: ## 生成代码覆盖率报告
	cargo llvm-cov --all-features --workspace --html

coverage-lcov: ## 生成 LCOV 格式的覆盖率报告
	cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

# 文档
docs: ## 生成文档
	cargo doc --all-features --no-deps

docs-open: ## 生成并打开文档
	cargo doc --all-features --no-deps --open

# 清理
clean: ## 清理构建文件
	cargo clean

clean-all: ## 清理所有文件（包括依赖缓存）
	cargo clean
	rm -rf ~/.cargo/registry/cache
	rm -rf ~/.cargo/git/db

# 安装
install: ## 安装到系统
	cargo install --path . --force

install-dev: ## 安装开发依赖
	rustup component add rustfmt clippy llvm-tools-preview
	cargo install cargo-watch cargo-audit cargo-llvm-cov criterion

# 开发环境
dev: ## 启动开发环境
	cargo watch -x 'run -- --debug'

dev-api: ## 启动开发环境并测试 API
	cargo watch -x 'run -- api "Hello, Claude!" --debug'

# Docker 相关
docker-build: ## 构建 Docker 镜像
	docker build -t claude-code-rust .

docker-build-dev: ## 构建开发 Docker 镜像
	docker build -f Dockerfile.dev -t claude-code-rust:dev .

docker-run: ## 运行 Docker 容器
	docker run -it --rm claude-code-rust

docker-dev: ## 启动开发环境
	docker-compose -f docker-compose.dev.yml up

docker-prod: ## 启动生产环境
	docker-compose up -d

docker-stop: ## 停止 Docker 环境
	docker-compose down

docker-logs: ## 查看 Docker 日志
	docker-compose logs -f

# 发布相关
release-patch: ## 发布补丁版本
	cargo release patch --execute

release-minor: ## 发布次要版本
	cargo release minor --execute

release-major: ## 发布主要版本
	cargo release major --execute

# 性能分析
profile: ## 性能分析
	cargo build --release
	perf record --call-graph=dwarf ./target/release/claude-code-rust api "Hello" --debug
	perf report

flamegraph: ## 生成火焰图
	cargo flamegraph --bin claude-code-rust -- api "Hello" --debug

# 数据库相关（如果使用）
db-setup: ## 设置数据库
	@echo "Setting up database..."
	# 这里可以添加数据库初始化命令

db-migrate: ## 运行数据库迁移
	@echo "Running database migrations..."
	# 这里可以添加迁移命令

db-reset: ## 重置数据库
	@echo "Resetting database..."
	# 这里可以添加重置命令

# 配置相关
config-init: ## 初始化配置文件
	./target/release/claude-code-rust config init --format yaml

config-show: ## 显示当前配置
	./target/release/claude-code-rust config show

config-validate: ## 验证配置文件
	./target/release/claude-code-rust config validate

# 工具链
toolchain-update: ## 更新 Rust 工具链
	rustup update

toolchain-nightly: ## 安装 nightly 工具链
	rustup toolchain install nightly

# 依赖管理
deps-update: ## 更新依赖
	cargo update

deps-outdated: ## 检查过时的依赖
	cargo outdated

deps-tree: ## 显示依赖树
	cargo tree

# 安全
security-check: ## 运行安全检查
	cargo audit
	cargo deny check

# 完整的 CI 流程
ci: format-check lint test audit ## 运行完整的 CI 检查

# 快速开发循环
quick: format lint test ## 快速开发检查

# 完整构建和测试
all: clean build test benchmark docs ## 完整构建流程

# 设置开发环境
setup: install-dev config-init ## 设置开发环境
	@echo "开发环境设置完成！"
	@echo "运行 'make dev' 开始开发"

# 变量
BINARY_NAME := claude-code-rust
VERSION := $(shell grep '^version' Cargo.toml | head -n1 | cut -d'"' -f2)

# 显示版本信息
version: ## 显示版本信息
	@echo "Claude Code Rust v$(VERSION)"
