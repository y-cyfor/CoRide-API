# CoRide-API Docker 部署 & 原生部署 完整方案

> 最后更新: 2026-04-17

---

## 一、项目概述

**CoRide-API** 是一个轻量级大模型 API 中转服务，允许用户通过统一的 OpenAI/Anthropic 兼容接口，代理转发到多个上游 AI 服务供应商。支持用户配额管理、多级限流、流量分发、调用日志等功能。

- 后端：Rust (Axum) + SQLite
- 前端：Vue 3 + TypeScript + Naive UI (SoybeanAdmin)
- 部署：Docker Compose / 原生一键脚本

---

## 二、当前架构现状审计

### 2.1 数据存储

| 数据类型 | 当前存储位置 | 存储方式 |
|---------|-------------|---------|
| 用户信息（账号、密码哈希、API Key、角色） | `data/coride.db` | SQLite `users` 表 |
| 渠道配置（供应商 BaseURL、API Keys、权重） | `data/coride.db` | SQLite `channels` 表 |
| 模型映射 | `data/coride.db` | SQLite `models` 表 |
| 配额配置 | `data/coride.db` | SQLite `quotas` 表 |
| 限流配置 | `data/coride.db` | SQLite `rate_limit_configs` 表 |
| 应用伪装预设 | `data/coride.db` | SQLite `app_profiles` 表 |
| 流量分发方案 | `data/coride.db` | SQLite `traffic_plans` 表 |
| 调用日志 | `data/coride.db` | SQLite `request_logs` 表 |
| 系统配置 | `config/config.yaml` | YAML 文件（只读，启动时加载） |

**重要发现**：当前所有数据（包括调用日志）都存储在 SQLite 数据库中，**没有** `backend/log` 目录的日志文件分割功能。

### 2.2 管理员密码存储

- **存储位置**：SQLite 数据库 `data/coride.db` 的 `users` 表
- **存储格式**：bcrypt 哈希（`password_hash` 字段）
- **初始化流程**：
  1. 启动时读取 `config/config.yaml` 中的 `admin.username` / `admin.password`
  2. 或被环境变量 `LP_ADMIN_USERNAME` / `LP_ADMIN_PASSWORD` 覆盖
  3. 调用 `ensure_admin()` 检查数据库是否存在该 admin 用户
  4. 不存在则 bcrypt 哈希密码后插入数据库
  5. **已存在则跳过**（不会覆盖已有密码）

### 2.3 当前环境变量支持（`config.rs`）

| 环境变量 | 说明 | 默认值 |
|---------|------|--------|
| `LP_PORT` | 服务端口 | `8000` |
| `LP_DB_PATH` | 数据库文件路径 | `./data/coride.db` |
| `LP_ADMIN_USERNAME` | 初始管理员用户名 | `admin` |
| `LP_ADMIN_PASSWORD` | 初始管理员密码 | `admin123` |
| `LP_JWT_SECRET` | JWT 签名密钥 | `change-me-to-random-string` |
| `LP_LOG_LEVEL` | 日志级别 | `info` |

### 2.4 当前 Docker 配置问题

| 文件 | 状态 | 说明 |
|------|------|------|
| `Dockerfile.backend` | ✅ CMD 路径已修正 | `/app/coride-api` |
| `Dockerfile.frontend` | ✅ 已修正依赖顺序 | monorepo pnpm install |
| `docker-compose.yml` | ⚠️ 缺少环境变量配置 | 需添加 environment 段 |
| `.dockerignore` | ✅ 已创建 | 排除 node_modules/target |

---

## 三、需求清单 & 实施方案

### 需求 1：一键部署脚本（原生构建）

**目标**：在没有 Docker 的服务器上，一条命令完成整个项目的部署。

#### 1.1 脚本功能清单

```bash
#!/bin/bash
# deploy.sh - CoRide-API 一键部署脚本

# 功能：
# 1. 检测系统环境（Rust/Node.js/pnpm/nginx 是否已安装）
# 2. 未安装则自动安装（支持 Ubuntu/Debian/CentOS）
# 3. 克隆/更新项目代码
# 4. 交互式设置管理员密码和 JWT Secret
# 5. 构建后端（cargo build --release）
# 6. 构建前端（pnpm install && pnpm build）
# 7. 配置 nginx 反向代理
# 8. 配置 systemd 服务（开机自启）
# 9. 初始化数据库和管理员账号
# 10. 启动服务并显示访问地址
```

#### 1.2 环境检测与安装

```bash
# 检测 Rust
if ! command -v cargo &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# 检测 Node.js >= 20
if ! command -v node &> /dev/null; then
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
    apt-get install -y nodejs
fi

# 检测 pnpm
if ! command -v pnpm &> /dev/null; then
    npm install -g pnpm
fi

# 检测 nginx
if ! command -v nginx &> /dev/null; then
    apt-get install -y nginx
fi
```

#### 1.3 交互式密码设置

```bash
echo "=== CoRide-API 初始配置 ==="
read -p "设置管理员用户名 (默认 admin): " ADMIN_USERNAME
ADMIN_USERNAME=${ADMIN_USERNAME:-admin}

read -s -p "设置管理员密码 (默认 admin123): " ADMIN_PASSWORD
ADMIN_PASSWORD=${ADMIN_PASSWORD:-admin123}
echo

# 生成 JWT Secret
read -p "生成随机 JWT Secret? (Y/n): " GEN_JWT
if [[ "$GEN_JWT" != "n" && "$GEN_JWT" != "N" ]]; then
    JWT_SECRET=$(openssl rand -hex 32)
else
    read -p "输入 JWT Secret: " JWT_SECRET
fi
```

#### 1.4 Systemd 服务配置

创建 `/etc/systemd/system/coride-api.service`：

```ini
[Unit]
Description=CoRide-API Backend
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=/opt/coride-api
ExecStart=/opt/coride-api/backend/target/release/coride-api
Restart=on-failure
RestartSec=5
Environment=CORIDE_ADMIN_USERNAME=admin
Environment=CORIDE_LOG_LEVEL=info
Environment=HOME=/opt/coride-api

[Install]
WantedBy=multi-user.target
```

#### 1.5 输出到 README

在 README 中添加一键部署章节：

```markdown
## 一键部署

### Docker 部署（推荐）
```bash
docker compose up -d
```

### 原生部署（无 Docker 环境）
```bash
curl -fsSL https://raw.githubusercontent.com/cyfor/coride-api/main/deploy.sh | bash
```
或下载脚本后执行：
```bash
chmod +x deploy.sh
./deploy.sh
```
```

---

### 需求 2：数据持久化方案

#### 2.1 数据库文件

**当前方案**：SQLite 文件 `data/coride.db`，已满足需求。

| 部署方式 | 存储位置 | 持久化方式 |
|---------|---------|-----------|
| 原生部署 | `./data/coride.db` | 直接在项目目录下 |
| Docker 部署 | `coride-data:/app/data` | Docker volume 映射 |

**Docker 映射到宿主机指定目录**（用户可选）：

```yaml
# docker-compose.yml - 方式 A：Docker 管理的 volume（推荐）
volumes:
  - coride-data:/app/data

# docker-compose.yml - 方式 B：映射到宿主机指定目录
volumes:
  - /opt/coride/data:/app/data
```

#### 2.2 调用日志文件分割（新增需求）

**现状**：调用日志存储在 SQLite `request_logs` 表中，没有文件日志。

**需求**：日志分割到 `backend/log` 目录，每天一个文件。

**实施方案**：

修改 `main.rs` 中的日志初始化，使用 `tracing-appender` 实现按天分割：

```rust
// Cargo.toml 新增依赖
tracing-appender = "0.2"

// main.rs 修改日志初始化
use tracing_appender::rolling::{RollingFileAppender, Rotation};

// 创建日志目录
std::fs::create_dir_all("log").ok();

// 按天分割的日志文件
let file_appender = RollingFileAppender::new(
    Rotation::DAILY,
    "log",
    "coride-api.log"
);

// 同时输出到 stdout 和文件
let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

tracing_subscriber::registry()
    .with(EnvFilter::new(format!("coride_api={log_level}")))
    .with(tracing_subscriber::fmt::Layer::default().with_writer(std::io::stdout))
    .with(tracing_subscriber::fmt::Layer::default().with_writer(non_blocking))
    .init();
```

**日志文件示例**：
```
backend/log/
├── coride-api.log.2026-04-17
├── coride-api.log.2026-04-18
└── coride-api.log.2026-04-19
```

**Docker 持久化**：

```yaml
volumes:
  - coride-data:/app/data
  - coride-logs:/app/log        # 新增日志持久化
  # 或映射到宿主机
  # - /opt/coride/logs:/app/log
```

---

### 需求 3：管理员密码配置方案

#### 3.1 密码存储位置

**回答**：管理员密码存储在 **SQLite 数据库**（`data/coride.db`）的 `users` 表中，以 bcrypt 哈希格式存储，**不是** JSON 文件。

```sql
-- users 表结构
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    username VARCHAR(64) UNIQUE NOT NULL,
    password_hash VARCHAR(128) NOT NULL,  -- bcrypt 哈希
    role VARCHAR(16) NOT NULL DEFAULT 'user',
    api_key VARCHAR(128) UNIQUE NOT NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'active',
    ...
);
```

#### 3.2 两种部署模式的密码配置

**模式 A：原生部署**

用户通过修改 `config/config.yaml` 设置初始密码：

```yaml
admin:
  username: "admin"
  password: "your-secure-password"
```

或者通过环境变量（适合 CI/CD）：

```bash
export CORIDE_ADMIN_USERNAME=admin
export CORIDE_ADMIN_PASSWORD=your-secure-password
```

**首次启动**：读取配置 → 创建 admin 用户 → 密码哈希存入数据库。
**后续启动**：数据库中已存在 admin → 跳过创建 → **config.yaml 中的密码不再生效**。

**修改密码方式**：
1. 通过管理面板修改（推荐）
2. 直接修改数据库（不推荐）
3. 删除数据库重新初始化（会丢失所有数据）

**模式 B：Docker 部署**

通过 `docker-compose.yml` 环境变量注入：

```yaml
services:
  backend:
    environment:
      - CORIDE_ADMIN_USERNAME=${CORIDE_ADMIN_USERNAME:-admin}
      - CORIDE_ADMIN_PASSWORD=${CORIDE_ADMIN_PASSWORD:-admin123}
```

用户需要准备 `.env` 文件：

```env
# .env
CORIDE_ADMIN_USERNAME=admin
CORIDE_ADMIN_PASSWORD=MySecurePassword123!
CORIDE_JWT_SECRET=your-random-secret-here
```

#### 3.3 环境变量前缀统一

当前代码使用 `LP_` 前缀（LiteProxy 遗留），建议统一改为 `CORIDE_`：

| 旧变量 | 新变量 |
|--------|--------|
| `LP_PORT` | `CORIDE_PORT` |
| `LP_DB_PATH` | `CORIDE_DB_PATH` |
| `LP_ADMIN_USERNAME` | `CORIDE_ADMIN_USERNAME` |
| `LP_ADMIN_PASSWORD` | `CORIDE_ADMIN_PASSWORD` |
| `LP_JWT_SECRET` | `CORIDE_JWT_SECRET` |
| `LP_LOG_LEVEL` | `CORIDE_LOG_LEVEL` |

**兼容性方案**：同时支持两种前缀，优先使用 `CORIDE_`，回退到 `LP_`。

---

## 四、实施清单

### Phase 1：基础修复（必须）

| # | 任务 | 文件 | 工作量 |
|---|------|------|--------|
| 1 | 统一环境变量前缀 `LP_` → `CORIDE_`（兼容旧前缀） | `backend/src/config.rs` | 小 |
| 2 | 添加日志文件分割（按天）到 `backend/log` | `backend/src/main.rs`, `Cargo.toml` | 中 |
| 3 | 更新 `docker-compose.yml` 环境变量配置 | `docker-compose.yml` | 小 |
| 4 | 创建 `.env.example` 模板 | `.env.example` | 小 |
| 5 | 更新 `Dockerfile.backend` 日志目录 | `Dockerfile.backend` | 小 |

### Phase 2：一键部署脚本

| # | 任务 | 文件 | 工作量 |
|---|------|------|--------|
| 6 | 创建 `deploy.sh` 一键部署脚本 | `deploy.sh` | 大 |
| 7 | 创建 systemd 服务模板 | `deploy/coride-api.service` | 小 |
| 8 | 创建 nginx 配置模板 | `deploy/nginx.conf` | 小 |
| 9 | 更新 README 添加部署说明 | `README.md` | 小 |

### Phase 3：Docker Hub 发布准备

| # | 任务 | 文件 | 工作量 |
|---|------|------|--------|
| 10 | 配置 GitHub Actions 自动构建 | `.github/workflows/docker.yml` | 中 |
| 11 | 更新 docker-compose.yml 支持 image 模式 | `docker-compose.yml` | 小 |

---

## 五、文件变更清单

### 需要修改的文件

| 文件 | 变更内容 |
|------|---------|
| `backend/src/config.rs` | 环境变量前缀改为 `CORIDE_`，兼容 `LP_` |
| `backend/src/main.rs` | 添加日志文件分割输出 |
| `backend/Cargo.toml` | 新增 `tracing-appender` 依赖 |
| `docker-compose.yml` | 添加环境变量、日志 volume |
| `Dockerfile.backend` | 添加日志目录 |

### 需要新建的文件

| 文件 | 说明 |
|------|------|
| `deploy.sh` | 一键部署脚本 |
| `deploy/coride-api.service` | systemd 服务配置 |
| `deploy/nginx.conf` | nginx 反向代理配置 |
| `.env.example` | 环境变量模板 |
| `.github/workflows/docker.yml` | GitHub Actions 自动构建 |

---

## 六、用户使用流程

### 场景 A：Docker 部署（推荐）

```bash
# 1. 克隆项目
git clone https://github.com/cyfor/coride-api.git
cd coride-api

# 2. 配置环境变量
cp .env.example .env
# 编辑 .env 设置密码

# 3. 一键启动
docker compose up -d

# 访问 http://服务器IP
# 管理员账号：admin / 你在 .env 中设置的密码
```

### 场景 B：原生部署（无 Docker）

```bash
# 1. 下载并执行一键脚本
curl -fsSL https://raw.githubusercontent.com/cyfor/coride-api/main/deploy.sh | bash

# 或手动执行
git clone https://github.com/cyfor/coride-api.git
cd coride-api
chmod +x deploy.sh
./deploy.sh

# 2. 脚本会提示设置管理员密码

# 访问 http://服务器IP:8000
```

### 场景 C：开发环境

```bash
# Windows
dev.bat

# Linux/Mac
cd backend && cargo run &
cd web && pnpm dev
```

---

## 七、数据持久化总结

| 数据类型 | 原生部署路径 | Docker 持久化方式 |
|---------|-------------|------------------|
| 数据库（用户/渠道/模型/配额/日志等全部数据） | `./data/coride.db` | `coride-data:/app/data` 或 `/自定义路径:/app/data` |
| 调用日志文件（新增） | `./backend/log/` | `coride-logs:/app/log` 或 `/自定义路径:/app/log` |
| 配置文件（只读模板） | `./backend/config/config.yaml` | `./backend/config:/app/config:ro` |
| 前端静态资源 | `./web/dist/` | 打包在 nginx 镜像中 |

**数据安全提示**：
- 备份 `data/` 目录即可备份所有业务数据
- `.env` 文件包含敏感信息，不要提交到 Git
- JWT Secret 泄露会导致所有 token 被伪造，务必使用随机字符串

---

## 八、上传 Docker Hub 完整步骤

### 8.1 前置准备

1. 注册 Docker Hub 账号：https://hub.docker.com
2. 创建仓库：`cyfor/coride-api-backend` 和 `cyfor/coride-api-frontend`
3. 本地登录：`docker login`

### 8.2 手动构建并推送

```bash
# 构建后端镜像（支持多平台）
docker buildx build --platform linux/amd64,linux/arm64 \
  -f Dockerfile.backend \
  -t cyfor/coride-api-backend:latest \
  -t cyfor/coride-api-backend:v0.1.0 \
  --push .

# 构建前端镜像
docker buildx build --platform linux/amd64,linux/arm64 \
  -f Dockerfile.frontend \
  -t cyfor/coride-api-frontend:latest \
  -t cyfor/coride-api-frontend:v0.1.0 \
  --push .
```

### 8.3 修改 docker-compose.yml 为 image 模式

```yaml
services:
  backend:
    image: cyfor/coride-api-backend:latest
    # build: ...  ← 注释掉或删除
    ...
  frontend:
    image: cyfor/coride-api-frontend:latest
    # build: ...  ← 注释掉或删除
    ...
```

### 8.4 用户一键部署（无需构建）

```bash
# 用户只需准备 .env 文件
docker compose up -d
```

### 8.5 自动化方案：GitHub Actions

创建 `.github/workflows/docker.yml`，实现 push 到 main 分支时自动构建并推送镜像。
