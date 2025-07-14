# 多阶段构建 Dockerfile for Claude Code Rust

# 构建阶段
FROM rust:1.75-slim as builder

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /app

# 复制 Cargo 文件
COPY Cargo.toml Cargo.lock ./

# 创建虚拟 main.rs 以缓存依赖
RUN mkdir src && echo "fn main() {}" > src/main.rs

# 构建依赖（缓存层）
RUN cargo build --release && rm -rf src

# 复制源代码
COPY src ./src
COPY benches ./benches
COPY tests ./tests

# 构建应用
RUN cargo build --release

# 运行时阶段
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 创建非 root 用户
RUN useradd -m -u 1000 claude && \
    mkdir -p /home/claude/.claude && \
    chown -R claude:claude /home/claude

# 复制构建的二进制文件
COPY --from=builder /app/target/release/claude-code-rust /usr/local/bin/claude-code-rust

# 设置权限
RUN chmod +x /usr/local/bin/claude-code-rust

# 切换到非 root 用户
USER claude
WORKDIR /home/claude

# 创建配置目录
RUN mkdir -p .claude/conversations .claude/memory .claude/logs

# 设置环境变量
ENV RUST_LOG=info
ENV CLAUDE_CONFIG_DIR=/home/claude/.claude

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD claude-code-rust --version || exit 1

# 暴露端口（如果将来添加 Web 服务）
EXPOSE 8080

# 默认命令
ENTRYPOINT ["claude-code-rust"]
CMD ["--help"]

# 标签
LABEL maintainer="your-email@example.com"
LABEL version="0.1.0"
LABEL description="Claude Code Rust - High-performance AI coding assistant"
LABEL org.opencontainers.image.source="https://github.com/your-org/claude-code-rust"
