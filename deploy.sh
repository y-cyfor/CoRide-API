#!/bin/bash
# CoRide-API 一键部署脚本
# 支持 Ubuntu/Debian/CentOS 原生部署

set -e

INSTALL_DIR="/opt/coride-api"
SERVICE_NAME="coride-api"

echo "========================================"
echo "  CoRide-API 一键部署脚本"
echo "========================================"
echo ""

# --- 颜色输出 ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC} $1"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# --- 检测系统 ---
detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS=$ID
    elif command -v yum &>/dev/null; then
        OS="centos"
    else
        OS="debian"
    fi
    info "检测到操作系统: $OS"
}

# --- 安装依赖 ---
install_deps() {
    info "检查并安装依赖..."

    # Rust
    if ! command -v cargo &> /dev/null; then
        warn "Rust 未安装，正在安装..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    else
        info "Rust 已安装: $(rustc --version)"
    fi

    # Node.js >= 20
    if ! command -v node &> /dev/null || [ "$(node -v | cut -d'v' -f2 | cut -d'.' -f1)" -lt 20 ]; then
        warn "Node.js 20+ 未安装，正在安装..."
        if [ "$OS" = "ubuntu" ] || [ "$OS" = "debian" ]; then
            curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
            apt-get install -y nodejs
        elif [ "$OS" = "centos" ]; then
            curl -fsSL https://rpm.nodesource.com/setup_20.x | bash -
            yum install -y nodejs
        fi
    else
        info "Node.js 已安装: $(node -v)"
    fi

    # pnpm
    if ! command -v pnpm &> /dev/null; then
        warn "pnpm 未安装，正在安装..."
        npm install -g pnpm
    else
        info "pnpm 已安装: $(pnpm --version)"
    fi

    # nginx
    if ! command -v nginx &> /dev/null; then
        warn "nginx 未安装，正在安装..."
        if [ "$OS" = "ubuntu" ] || [ "$OS" = "debian" ]; then
            apt-get install -y nginx
        elif [ "$OS" = "centos" ]; then
            yum install -y nginx
        fi
    else
        info "nginx 已安装"
    fi
}

# --- 交互式配置 ---
interactive_config() {
    echo ""
    echo "=== 初始配置 ==="
    read -p "设置管理员用户名 (默认 admin): " ADMIN_USERNAME
    ADMIN_USERNAME=${ADMIN_USERNAME:-admin}

    read -s -p "设置管理员密码 (默认 admin123): " ADMIN_PASSWORD
    ADMIN_PASSWORD=${ADMIN_PASSWORD:-admin123}
    echo ""

    read -p "生成随机 JWT Secret? (Y/n): " GEN_JWT
    if [[ "$GEN_JWT" != "n" && "$GEN_JWT" != "N" ]]; then
        if command -v openssl &> /dev/null; then
            JWT_SECRET=$(openssl rand -hex 32)
        else
            JWT_SECRET=$(head -c 32 /dev/urandom | md5sum | awk '{print $1}')
        fi
        info "JWT Secret 已生成: $JWT_SECRET"
    else
        read -p "输入 JWT Secret: " JWT_SECRET
    fi

    read -p "服务端口 (默认 8000): " SERVER_PORT
    SERVER_PORT=${SERVER_PORT:-8000}
}

# --- 部署代码 ---
deploy_code() {
    info "部署代码到 $INSTALL_DIR"
    mkdir -p "$INSTALL_DIR"

    # 复制当前目录到安装目录
    rsync -a --exclude='node_modules' --exclude='target' --exclude='.git' \
        "$(dirname "$0")/" "$INSTALL_DIR/"

    # 创建数据目录
    mkdir -p "$INSTALL_DIR/data"
    mkdir -p "$INSTALL_DIR/log"
}

# --- 构建项目 ---
build_project() {
    info "构建后端..."
    cd "$INSTALL_DIR/backend"
    cargo build --release

    info "构建前端..."
    cd "$INSTALL_DIR/web"
    pnpm install --frozen-lockfile
    pnpm build
}

# --- 配置 nginx ---
configure_nginx() {
    info "配置 nginx 反向代理..."

    cat > /etc/nginx/sites-available/$SERVICE_NAME <<NGINX_EOF
server {
    listen 80;
    server_name _;

    root $INSTALL_DIR/web/dist;
    index index.html;

    location / {
        try_files \$uri \$uri/ /index.html;
    }

    location /admin/ {
        proxy_pass http://127.0.0.1:${SERVER_PORT};
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }

    location /v1/ {
        proxy_pass http://127.0.0.1:${SERVER_PORT};
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }

    location /health {
        proxy_pass http://127.0.0.1:${SERVER_PORT};
    }

    gzip on;
    gzip_types text/plain text/css application/json application/javascript;
    gzip_min_length 256;
}
NGINX_EOF

    ln -sf /etc/nginx/sites-available/$SERVICE_NAME /etc/nginx/sites-enabled/$SERVICE_NAME
    nginx -t && systemctl reload nginx
    info "nginx 配置完成"
}

# --- 配置 systemd ---
configure_systemd() {
    info "配置 systemd 服务..."

    cat > /etc/systemd/system/$SERVICE_NAME.service <<SYSTEMD_EOF
[Unit]
Description=CoRide-API Backend
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/backend/target/release/coride-api
Restart=on-failure
RestartSec=5
Environment=CORIDE_ADMIN_USERNAME=$ADMIN_USERNAME
Environment=CORIDE_ADMIN_PASSWORD=$ADMIN_PASSWORD
Environment=CORIDE_JWT_SECRET=$JWT_SECRET
Environment=CORIDE_LOG_LEVEL=info
Environment=HOME=$INSTALL_DIR

[Install]
WantedBy=multi-user.target
SYSTEMD_EOF

    systemctl daemon-reload
    systemctl enable $SERVICE_NAME
    systemctl restart $SERVICE_NAME
    info "systemd 服务已启动"
}

# --- 主流程 ---
detect_os

# 检查是否有 root 权限
if [ "$EUID" -ne 0 ]; then
    error "请使用 root 用户或 sudo 执行此脚本"
fi

install_deps
interactive_config
deploy_code
build_project
configure_nginx
configure_systemd

echo ""
echo "========================================"
echo "  部署完成！"
echo "========================================"
echo ""
echo "访问地址: http://$(hostname -I | awk '{print $1}')"
echo "管理员账号: $ADMIN_USERNAME / $ADMIN_PASSWORD"
echo ""
echo "服务管理命令:"
echo "  查看状态:  systemctl status $SERVICE_NAME"
echo "  重启服务:  systemctl restart $SERVICE_NAME"
echo "  查看日志:  journalctl -u $SERVICE_NAME -f"
echo "  nginx 日志: tail -f /var/log/nginx/access.log"
