#!/bin/bash

# Claude Code Rust 开发环境快速设置脚本

set -euo pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 函数定义
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${PURPLE}[STEP]${NC} $1"
}

# 显示欢迎信息
show_welcome() {
    echo -e "${CYAN}"
    cat << "EOF"
    ╔═══════════════════════════════════════════════════════════════╗
    ║                                                               ║
    ║               🦀 Claude Code Rust 🦀                         ║
    ║                                                               ║
    ║            Development Environment Setup Script               ║
    ║                                                               ║
    ╚═══════════════════════════════════════════════════════════════╝
EOF
    echo -e "${NC}"
}

# 检查系统要求
check_system_requirements() {
    log_step "Checking system requirements..."
    
    # 检查操作系统
    case "$(uname -s)" in
        Linux*)     MACHINE=Linux;;
        Darwin*)    MACHINE=Mac;;
        CYGWIN*)    MACHINE=Cygwin;;
        MINGW*)     MACHINE=MinGw;;
        *)          MACHINE="UNKNOWN:$(uname -s)"
    esac
    
    log_info "Detected system: $MACHINE"
    
    # 检查 Rust
    if command -v rustc &> /dev/null; then
        local rust_version=$(rustc --version)
        log_success "Rust found: $rust_version"
    else
        log_error "Rust not found. Please install Rust from https://rustup.rs/"
        exit 1
    fi
    
    # 检查 Cargo
    if command -v cargo &> /dev/null; then
        local cargo_version=$(cargo --version)
        log_success "Cargo found: $cargo_version"
    else
        log_error "Cargo not found. Please install Rust toolchain."
        exit 1
    fi
    
    # 检查 Git
    if command -v git &> /dev/null; then
        local git_version=$(git --version)
        log_success "Git found: $git_version"
    else
        log_warning "Git not found. Some features may not work properly."
    fi
}

# 安装 Rust 开发工具
install_rust_tools() {
    log_step "Installing Rust development tools..."
    
    # 安装组件
    log_info "Installing Rust components..."
    rustup component add rustfmt clippy llvm-tools-preview
    
    # 安装开发工具
    log_info "Installing development tools..."
    local tools=(
        "cargo-watch"
        "cargo-audit" 
        "cargo-llvm-cov"
        "cargo-deny"
        "cargo-outdated"
        "cargo-tree"
        "cargo-expand"
        "cargo-flamegraph"
    )
    
    for tool in "${tools[@]}"; do
        if ! cargo install --list | grep -q "^$tool "; then
            log_info "Installing $tool..."
            cargo install "$tool" || log_warning "Failed to install $tool"
        else
            log_success "$tool already installed"
        fi
    done
}

# 设置项目配置
setup_project_config() {
    log_step "Setting up project configuration..."
    
    # 创建配置目录
    mkdir -p config
    
    # 创建开发配置文件
    if [[ ! -f "config/development.yaml" ]]; then
        log_info "Creating development configuration..."
        cat > config/development.yaml << EOF
api:
  anthropic_api_key: ""  # 设置你的 API 密钥
  base_url: "https://api.anthropic.com"
  default_model: "claude-3-haiku-20240307"
  max_tokens: 4096
  temperature: 0.7
  top_p: 0.9
  top_k: 40
  stream: true

logging:
  level: "debug"
  console: true
  structured: false
  file: "logs/claude-code-rust.log"

preferences:
  editor: "code"
  shell: "/bin/zsh"
  enable_autocomplete: true
  enable_syntax_highlighting: true

web:
  port: 8080
  host: "127.0.0.1"
  enable_cors: true
  static_dir: "web/static"
  enable_compression: true

performance:
  enable_monitoring: true
  metrics_interval: 60
  max_memory_mb: 512
  max_concurrent_requests: 100
EOF
        log_success "Development configuration created"
    else
        log_info "Development configuration already exists"
    fi
    
    # 创建日志目录
    mkdir -p logs
    
    # 创建 Web 静态文件目录
    mkdir -p web/static
}

# 设置环境变量
setup_environment_variables() {
    log_step "Setting up environment variables..."
    
    # 创建 .env 文件
    if [[ ! -f ".env" ]]; then
        log_info "Creating .env file..."
        cat > .env << EOF
# Claude Code Rust Environment Variables

# API Configuration
ANTHROPIC_API_KEY=your_api_key_here
CLAUDE_CONFIG_DIR=./config

# Logging
RUST_LOG=debug
RUST_BACKTRACE=1

# Development
CARGO_TARGET_DIR=./target
CARGO_INCREMENTAL=1

# Web Server
WEB_PORT=8080
WEB_HOST=127.0.0.1

# Performance
ENABLE_PERFORMANCE_MONITORING=true
EOF
        log_success ".env file created"
        log_warning "Please update .env file with your actual API key"
    else
        log_info ".env file already exists"
    fi
}

# 构建项目
build_project() {
    log_step "Building project..."
    
    # 检查依赖
    log_info "Checking dependencies..."
    cargo check
    
    # 构建开发版本
    log_info "Building development version..."
    cargo build
    
    # 构建发布版本
    log_info "Building release version..."
    cargo build --release
    
    log_success "Project built successfully"
}

# 运行测试
run_tests() {
    log_step "Running tests..."
    
    # 运行单元测试
    log_info "Running unit tests..."
    cargo test --lib
    
    # 运行集成测试
    log_info "Running integration tests..."
    cargo test --test integration_tests || log_warning "Some integration tests failed"
    
    log_success "Tests completed"
}

# 设置 Git hooks
setup_git_hooks() {
    if [[ -d ".git" ]]; then
        log_step "Setting up Git hooks..."
        
        # 创建 pre-commit hook
        cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
# Pre-commit hook for Claude Code Rust

set -e

echo "Running pre-commit checks..."

# Format check
echo "Checking code formatting..."
cargo fmt --all -- --check

# Clippy check
echo "Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings

# Tests
echo "Running tests..."
cargo test --lib

echo "Pre-commit checks passed!"
EOF
        
        chmod +x .git/hooks/pre-commit
        log_success "Git hooks configured"
    else
        log_info "Not a Git repository, skipping Git hooks setup"
    fi
}

# 创建开发脚本
create_dev_scripts() {
    log_step "Creating development scripts..."
    
    mkdir -p scripts
    
    # 创建快速启动脚本
    cat > scripts/dev.sh << 'EOF'
#!/bin/bash
# Quick development start script

echo "🚀 Starting Claude Code Rust development server..."

# 加载环境变量
if [[ -f ".env" ]]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# 启动开发服务器
cargo watch -x 'run -- --debug'
EOF
    
    # 创建测试脚本
    cat > scripts/test.sh << 'EOF'
#!/bin/bash
# Test runner script

echo "🧪 Running Claude Code Rust tests..."

# 运行所有测试
cargo test --all-features

# 生成覆盖率报告
echo "📊 Generating coverage report..."
cargo llvm-cov --all-features --workspace --html
echo "Coverage report generated in target/llvm-cov/html/"
EOF
    
    # 创建基准测试脚本
    cat > scripts/bench.sh << 'EOF'
#!/bin/bash
# Benchmark runner script

echo "⚡ Running Claude Code Rust benchmarks..."

# 运行基准测试
cargo bench

echo "📈 Benchmark results saved in target/criterion/"
EOF
    
    # 创建代码质量检查脚本
    cat > scripts/quality.sh << 'EOF'
#!/bin/bash
# Code quality check script

echo "🔍 Running code quality checks..."

# Format check
echo "📝 Checking formatting..."
cargo fmt --all -- --check

# Clippy
echo "📎 Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings

# Security audit
echo "🔒 Running security audit..."
cargo audit

# Dependency check
echo "📦 Checking dependencies..."
cargo outdated

echo "✅ Quality checks completed!"
EOF
    
    # 给脚本添加执行权限
    chmod +x scripts/*.sh
    
    log_success "Development scripts created"
}

# 显示完成信息
show_completion_info() {
    log_success "Development environment setup completed!"
    echo
    echo -e "${CYAN}🎉 You're all set! Here's what you can do next:${NC}"
    echo
    echo -e "${GREEN}📝 Configuration:${NC}"
    echo "  • Edit config/development.yaml with your settings"
    echo "  • Update .env file with your API key"
    echo
    echo -e "${GREEN}🚀 Development:${NC}"
    echo "  • Run: ./scripts/dev.sh (start development server)"
    echo "  • Run: make dev (alternative using Makefile)"
    echo "  • Run: cargo run -- --help (see all commands)"
    echo
    echo -e "${GREEN}🧪 Testing:${NC}"
    echo "  • Run: ./scripts/test.sh (run all tests)"
    echo "  • Run: ./scripts/bench.sh (run benchmarks)"
    echo "  • Run: make test (alternative using Makefile)"
    echo
    echo -e "${GREEN}🔍 Quality:${NC}"
    echo "  • Run: ./scripts/quality.sh (code quality checks)"
    echo "  • Run: make lint (linting)"
    echo "  • Run: make format (code formatting)"
    echo
    echo -e "${GREEN}🌐 Web Interface:${NC}"
    echo "  • Run: cargo run --features web-server -- serve"
    echo "  • Visit: http://localhost:8080"
    echo "  • Dashboard: http://localhost:8080/dashboard"
    echo
    echo -e "${GREEN}📚 Documentation:${NC}"
    echo "  • Run: cargo doc --open (generate and open docs)"
    echo "  • Read: README.md, IMPLEMENTATION_STATUS.md"
    echo
    echo -e "${YELLOW}⚠️  Don't forget to:${NC}"
    echo "  • Set your ANTHROPIC_API_KEY in .env file"
    echo "  • Review and customize config/development.yaml"
    echo "  • Install any additional system dependencies if needed"
    echo
    echo -e "${PURPLE}Happy coding! 🦀✨${NC}"
}

# 主函数
main() {
    show_welcome
    
    check_system_requirements
    install_rust_tools
    setup_project_config
    setup_environment_variables
    build_project
    run_tests
    setup_git_hooks
    create_dev_scripts
    
    show_completion_info
}

# 脚本入口
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
