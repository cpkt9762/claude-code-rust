# Claude Code Rust

一个用 Rust 实现的 Claude Code 工具 - 强大的代理编程助手。

## 🚀 特性

- **完整的CLI界面**: 支持多种命令和子命令
- **配置管理**: 灵活的YAML配置系统
- **MCP协议支持**: Model Context Protocol 集成
- **文件系统管理**: 异步文件操作
- **Git集成**: 版本控制操作
- **网络功能**: HTTP客户端和API集成
- **进程管理**: 子进程执行和管理
- **终端UI**: 丰富的终端用户界面
- **成本跟踪**: API使用成本监控
- **权限管理**: 细粒度的操作权限控制

## 📦 安装

### 从源码构建

```bash
git clone https://github.com/anthropics/claude-code-rust.git
cd claude-code-rust
cargo build --release
```

### 运行

```bash
# 显示帮助
./target/release/claude-code-rust --help

# 显示版本
./target/release/claude-code-rust --version

# 启动交互模式
./target/release/claude-code-rust interactive
```

## 🛠️ 使用方法

### 基本命令

```bash
# 配置管理
claude-code-rust config show
claude-code-rust config set api_key your_key

# MCP服务器管理
claude-code-rust mcp start --config server.json
claude-code-rust mcp list

# Git操作
claude-code-rust git status
claude-code-rust git commit -m "message"

# 进程管理
claude-code-rust process list
claude-code-rust process kill <id>

# 成本查看
claude-code-rust cost today
claude-code-rust cost stats --days 30
```

### 配置文件

配置文件位于 `~/.config/claude-code-rust/config.yaml`:

```yaml
api:
  base_url: "https://api.anthropic.com"
  timeout_seconds: 30
  max_retries: 3

ui:
  theme: "dark"
  enable_tui: false

permissions:
  require_confirmation: true
  allowed_tools:
    - "file_read"
    - "file_write"
    - "network_request"
  denied_tools: []

memory:
  max_conversations: 100
  auto_save: true
```

## 🧪 测试

项目包含完整的测试套件：

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test --lib                    # 库测试
cargo test --test unit_tests        # 单元测试
cargo test --test integration_tests # 集成测试

# 详细输出
cargo test -- --nocapture
```

### 测试统计
- **库测试**: 12个测试用例
- **单元测试**: 13个测试用例  
- **集成测试**: 13个测试用例
- **总计**: 47个测试用例，全部通过 ✅

## 🏗️ 架构

### 模块结构

```
src/
├── cli/           # 命令行接口
├── config/        # 配置管理
├── conversation/  # 对话管理
├── cost/          # 成本跟踪
├── error/         # 错误处理
├── fs/            # 文件系统
├── git/           # Git集成
├── mcp/           # MCP协议
├── network/       # 网络功能
├── process/       # 进程管理
├── ui/            # 用户界面
├── lib.rs         # 库入口
└── main.rs        # 主程序
```

### 可选特性

```toml
[features]
default = []
image-processing = ["image"]
syntax-highlighting = ["syntect"]
```

## 🔧 开发

### 环境要求

- Rust 1.70+
- Cargo
- Git

### 开发命令

```bash
# 开发构建
cargo build

# 运行检查
cargo check

# 格式化代码
cargo fmt

# 代码检查
cargo clippy

# 生成文档
cargo doc --open
```

## 📝 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件。

## 🤝 贡献

欢迎贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详情。

## 📚 文档

- [测试总结](TESTING_SUMMARY.md)
- [API文档](https://docs.rs/claude-code-rust)
- [用户指南](docs/user-guide.md)

## 🔗 相关链接

- [Claude Code (JavaScript版本)](https://github.com/anthropics/claude-code)
- [Anthropic API文档](https://docs.anthropic.com/)
- [MCP协议规范](https://modelcontextprotocol.io/)

---

**注意**: 这是 Claude Code 的 Rust 实现版本，旨在提供更好的性能和系统集成能力。
