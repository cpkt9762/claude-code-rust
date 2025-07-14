#!/bin/bash

# Claude Code Rust 部署脚本
# 用于自动化部署到生产环境

set -euo pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 配置变量
APP_NAME="claude-code-rust"
DEPLOY_USER="claude"
DEPLOY_PATH="/opt/claude-code-rust"
SERVICE_NAME="claude-code-rust"
BACKUP_PATH="/opt/backups/claude-code-rust"
LOG_PATH="/var/log/claude-code-rust"

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

# 检查是否为 root 用户
check_root() {
    if [[ $EUID -eq 0 ]]; then
        log_error "This script should not be run as root"
        exit 1
    fi
}

# 检查依赖
check_dependencies() {
    log_info "Checking dependencies..."
    
    local deps=("cargo" "git" "systemctl" "nginx")
    for dep in "${deps[@]}"; do
        if ! command -v "$dep" &> /dev/null; then
            log_error "Required dependency '$dep' is not installed"
            exit 1
        fi
    done
    
    log_success "All dependencies are available"
}

# 创建部署用户和目录
setup_environment() {
    log_info "Setting up deployment environment..."
    
    # 创建部署用户（如果不存在）
    if ! id "$DEPLOY_USER" &>/dev/null; then
        sudo useradd -r -s /bin/bash -d "$DEPLOY_PATH" "$DEPLOY_USER"
        log_success "Created deploy user: $DEPLOY_USER"
    fi
    
    # 创建目录
    sudo mkdir -p "$DEPLOY_PATH" "$BACKUP_PATH" "$LOG_PATH"
    sudo chown -R "$DEPLOY_USER:$DEPLOY_USER" "$DEPLOY_PATH" "$BACKUP_PATH" "$LOG_PATH"
    
    log_success "Environment setup completed"
}

# 构建应用
build_application() {
    log_info "Building application..."
    
    # 清理之前的构建
    cargo clean
    
    # 构建发布版本
    cargo build --release --all-features
    
    # 运行测试
    log_info "Running tests..."
    cargo test --release
    
    log_success "Application built successfully"
}

# 备份当前版本
backup_current_version() {
    if [[ -f "$DEPLOY_PATH/$APP_NAME" ]]; then
        log_info "Backing up current version..."
        
        local backup_file="$BACKUP_PATH/${APP_NAME}-$(date +%Y%m%d-%H%M%S).tar.gz"
        sudo tar -czf "$backup_file" -C "$DEPLOY_PATH" .
        
        log_success "Backup created: $backup_file"
    else
        log_info "No existing version to backup"
    fi
}

# 部署新版本
deploy_new_version() {
    log_info "Deploying new version..."
    
    # 停止服务
    if sudo systemctl is-active --quiet "$SERVICE_NAME"; then
        log_info "Stopping service..."
        sudo systemctl stop "$SERVICE_NAME"
    fi
    
    # 复制新的二进制文件
    sudo cp "target/release/$APP_NAME" "$DEPLOY_PATH/"
    sudo chown "$DEPLOY_USER:$DEPLOY_USER" "$DEPLOY_PATH/$APP_NAME"
    sudo chmod +x "$DEPLOY_PATH/$APP_NAME"
    
    # 复制配置文件（如果存在）
    if [[ -f "config/production.yaml" ]]; then
        sudo cp "config/production.yaml" "$DEPLOY_PATH/config.yaml"
        sudo chown "$DEPLOY_USER:$DEPLOY_USER" "$DEPLOY_PATH/config.yaml"
    fi
    
    # 复制 Web 静态文件（如果存在）
    if [[ -d "web/static" ]]; then
        sudo cp -r "web/static" "$DEPLOY_PATH/"
        sudo chown -R "$DEPLOY_USER:$DEPLOY_USER" "$DEPLOY_PATH/static"
    fi
    
    log_success "New version deployed"
}

# 创建 systemd 服务文件
create_systemd_service() {
    log_info "Creating systemd service..."
    
    sudo tee "/etc/systemd/system/${SERVICE_NAME}.service" > /dev/null <<EOF
[Unit]
Description=Claude Code Rust - AI Coding Assistant
After=network.target
Wants=network.target

[Service]
Type=simple
User=$DEPLOY_USER
Group=$DEPLOY_USER
WorkingDirectory=$DEPLOY_PATH
ExecStart=$DEPLOY_PATH/$APP_NAME serve --host 0.0.0.0 --port 8080
ExecReload=/bin/kill -HUP \$MAINPID
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=$SERVICE_NAME

# 环境变量
Environment=RUST_LOG=info
Environment=CLAUDE_CONFIG_DIR=$DEPLOY_PATH

# 安全设置
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$DEPLOY_PATH $LOG_PATH

# 资源限制
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
EOF

    sudo systemctl daemon-reload
    sudo systemctl enable "$SERVICE_NAME"
    
    log_success "Systemd service created and enabled"
}

# 配置 Nginx 反向代理
configure_nginx() {
    log_info "Configuring Nginx..."
    
    sudo tee "/etc/nginx/sites-available/$APP_NAME" > /dev/null <<EOF
server {
    listen 80;
    server_name claude-code.example.com;  # 替换为你的域名
    
    # 重定向到 HTTPS
    return 301 https://\$server_name\$request_uri;
}

server {
    listen 443 ssl http2;
    server_name claude-code.example.com;  # 替换为你的域名
    
    # SSL 配置（需要配置证书）
    # ssl_certificate /path/to/certificate.crt;
    # ssl_certificate_key /path/to/private.key;
    
    # 安全头
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    
    # 代理到应用
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_cache_bypass \$http_upgrade;
        
        # 超时设置
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }
    
    # 静态文件
    location /static/ {
        alias $DEPLOY_PATH/static/;
        expires 1y;
        add_header Cache-Control "public, immutable";
    }
    
    # 健康检查
    location /health {
        access_log off;
        proxy_pass http://127.0.0.1:8080/health;
    }
}
EOF

    # 启用站点
    sudo ln -sf "/etc/nginx/sites-available/$APP_NAME" "/etc/nginx/sites-enabled/"
    
    # 测试配置
    sudo nginx -t
    
    # 重新加载 Nginx
    sudo systemctl reload nginx
    
    log_success "Nginx configured successfully"
}

# 启动服务
start_service() {
    log_info "Starting service..."
    
    sudo systemctl start "$SERVICE_NAME"
    
    # 等待服务启动
    sleep 5
    
    # 检查服务状态
    if sudo systemctl is-active --quiet "$SERVICE_NAME"; then
        log_success "Service started successfully"
    else
        log_error "Failed to start service"
        sudo systemctl status "$SERVICE_NAME"
        exit 1
    fi
}

# 健康检查
health_check() {
    log_info "Performing health check..."
    
    local max_attempts=30
    local attempt=1
    
    while [[ $attempt -le $max_attempts ]]; do
        if curl -f -s http://localhost:8080/health > /dev/null; then
            log_success "Health check passed"
            return 0
        fi
        
        log_info "Health check attempt $attempt/$max_attempts failed, retrying..."
        sleep 2
        ((attempt++))
    done
    
    log_error "Health check failed after $max_attempts attempts"
    return 1
}

# 显示部署信息
show_deployment_info() {
    log_success "Deployment completed successfully!"
    echo
    echo "Service Information:"
    echo "  Name: $SERVICE_NAME"
    echo "  Status: $(sudo systemctl is-active $SERVICE_NAME)"
    echo "  Path: $DEPLOY_PATH"
    echo "  Logs: journalctl -u $SERVICE_NAME -f"
    echo
    echo "Web Interface:"
    echo "  Local: http://localhost:8080"
    echo "  Dashboard: http://localhost:8080/dashboard"
    echo "  API: http://localhost:8080/api/status"
    echo
    echo "Management Commands:"
    echo "  Start: sudo systemctl start $SERVICE_NAME"
    echo "  Stop: sudo systemctl stop $SERVICE_NAME"
    echo "  Restart: sudo systemctl restart $SERVICE_NAME"
    echo "  Status: sudo systemctl status $SERVICE_NAME"
    echo "  Logs: sudo journalctl -u $SERVICE_NAME -f"
}

# 主函数
main() {
    log_info "Starting Claude Code Rust deployment..."
    
    check_root
    check_dependencies
    setup_environment
    build_application
    backup_current_version
    deploy_new_version
    create_systemd_service
    
    # 可选：配置 Nginx（如果需要）
    if command -v nginx &> /dev/null; then
        read -p "Configure Nginx reverse proxy? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            configure_nginx
        fi
    fi
    
    start_service
    
    if health_check; then
        show_deployment_info
    else
        log_error "Deployment failed health check"
        exit 1
    fi
}

# 脚本入口
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
