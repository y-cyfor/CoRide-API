# CoRide-API

> 一个轻量级 AI API 代理服务 —— 多模型拼车中转

<div align="center">

[English](README-en.md)

[![License](https://img.shields.io/badge/license-Apache--2.0-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)
[![Vue](https://img.shields.io/badge/vue-3-green.svg)](https://vuejs.org/)
[![Docker](https://img.shields.io/badge/Docker-cyfor%2Fcoride--api-blue)](https://hub.docker.com/r/cyfor/coride-api)

</div>

---

## 项目简介

CoRide-API 是一个基于 Rust + Vue 3 构建的轻量级 AI API 代理管理工具。它允许多个用户共享一组上游 AI 服务渠道（如 OpenAI、Anthropic、阿里云通义千问、智谱 AI、Kimi 等），通过统一的 OpenAI / anthropics 兼容接口对外提供服务。

CoRide-API is a lightweight AI API proxy management tool built with Rust + Vue 3. It allows multiple users to share a set of upstream AI service channels (such as OpenAI, Anthropic, Alibaba Cloud Tongyi Qianwen, Zhipu AI, Kimi, etc.) through a unified OpenAI-compatible interface.

**核心业务场景：**

- 管理员集中管理多个 AI 服务渠道的 API Key 和配置
  Administrators centrally manage API Keys and configurations of multiple AI service channels
- 多个用户通过各自的 API Key 接入代理服务
  Multiple users access the proxy service through their individual API Keys
- 按用户进行配额限制（请求数 / Token 数）和速率控制
  Per-user quota limits (requests / tokens) and rate control
- 支持按用户绑定可用模型，实现精细化的权限管理
  Model binding per user for fine-grained access control
- 分时段、按比例将请求分流到不同的应用预设（伪装 UA 和请求头）
  Time-based, weighted request routing to different app presets (UA + header spoofing)
- 全量请求日志记录与统计分析，方便运维监控
  Full request logging and statistics for operational monitoring

**项目定位：** 纯无商业化功能的个人/小团队拼车工具，专注于简洁、高效、易部署。
**Positioning:** A personal/small-team carpooling tool with no commercial features, focused on simplicity, efficiency, and easy deployment.

---

## 开发方式

本项目开发者本职为产品经理，项目绝大部分代码（超过99%）使用 **Vibe Coding** 方式完成 —— 通过自然语言描述需求，由 AI 编写代码实现。

| 项目 | 工具/模型 |
|------|-----------|
| 编程工具 | [Claude Code](https://github.com/anthropics/claude-code)、[OpenCode](https://github.com/opencode-ai/opencode) |
| 大语言模型 | Qwen 3.6-plus、小米 MiMo v2 Pro、 智谱GLM-5.1 |

---

## 安装与使用

### 前置要求

- **Rust 1.88+**（后端）
- **Node.js 20+** + pnpm（前端）
- **Docker & Docker Compose**（可选，推荐）

### 方法一：从 Docker Hub 镜像部署（推荐）

```bash
docker pull cyfor/coride-api:latest

# 创建 docker-compose.yml 和 .env
mkdir -p /opt/coride
cat > /opt/coride/.env << 'EOF'
CORIDE_ADMIN_USERNAME=admin
CORIDE_ADMIN_PASSWORD=你的密码
CORIDE_JWT_SECRET=你的随机密钥
CORIDE_LOG_LEVEL=info
EOF

cat > /opt/coride/docker-compose.yml << 'EOF'
services:
  coride-api:
    image: cyfor/coride-api:latest
    container_name: coride-api
    restart: unless-stopped
    ports:
      - "80:80"
    volumes:
      - ./data:/app/data
      - ./logs:/app/log
    env_file:
      - .env
    environment:
      - CORIDE_DB_PATH=/app/data/coride.db
EOF

cd /opt/coride && docker compose up -d
```

启动成功后访问 `http://服务器IP`，使用你在 `.env` 中设置的管理员账号登录。

> **端口说明**：
> - Docker 容器内：nginx 监听 **80** 端口（反向代理到后端 8000 端口）
> - `"80:80"` 格式为 `"宿主机端口:容器端口"`
> - 如需更换端口，修改第一个数字即可，如 `"8080:80"` 则通过 `http://IP:8080` 访问
> - **宝塔面板部署**：容器端口填 **80**，主机端口自定义，然后反向代理到 `http://127.0.0.1:主机端口`
> - ⚠️ 9527 仅是开发环境 Vite 端口，Docker 部署不使用

### 方法二：开发环境启动

```bash
# 后端
cd backend && cargo run

# 前端（新终端）
cd web && pnpm install && pnpm dev
```

默认管理员账号: `admin` / `admin123`

> 请在生产环境中通过环境变量 `CORIDE_ADMIN_PASSWORD` 和 `CORIDE_JWT_SECRET` 修改默认凭据。

---

## 项目结构

```
CoRide-API/
├── backend/
│   ├── src/
│   │   ├── main.rs                  # 应用入口：路由注册、后台任务、优雅关闭
│   │   ├── lib.rs                   # 库入口：AppState 定义
│   │   ├── config.rs                # 配置加载（YAML + 环境变量覆盖）
│   │   ├── state/app_state.rs       # 应用状态构建
│   │   ├── db/
│   │   │   ├── mod.rs               # 连接池初始化 + 迁移执行
│   │   │   ├── migrations/          # SQL 迁移文件（12 个）
│   │   │   └── models.rs            # 所有结构体 + CRUD 函数
│   │   ├── middleware/
│   │   │   ├── auth.rs              # 用户 API Key 认证 + 模型权限检查
│   │   │   ├── admin_auth.rs        # 管理员 JWT 认证 + 角色检查
│   │   │   ├── user_auth.rs         # 用户 JWT 认证（无需 admin 角色）
│   │   │   ├── rate_limit.rs        # 全局 QPS/并发限制
│   │   │   └── ip_filter.rs         # IP 访问控制（全局黑名单 + 用户白名单）
│   │   ├── router/
│   │   │   ├── proxy_routes.rs      # 代理请求处理 + 用户自助查询
│   │   │   └── admin_routes.rs      # 管理后台 CRUD + 统计 + 用户端路由
│   │   ├── service/
│   │   │   ├── proxy.rs             # HTTP 代理（流式/非流式）
│   │   │   ├── openai.rs            # OpenAI 格式适配
│   │   │   ├── anthropic.rs         # Anthropic 格式适配
│   │   │   ├── quota.rs             # 配额检查与扣除
│   │   │   ├── health.rs            # 渠道健康检查
│   │   │   └── log.rs               # 日志记录
│   │   └── utils/
│   │       ├── jwt.rs               # JWT 令牌签发与验证
│   │       └── token_counter.rs     # Token 估算
│   └── config/config.yaml           # 配置文件
├── web/
│   └── src/
│       ├── views/
│       │   ├── home/                # 仪表盘（统计卡片 + 图表）
│       │   ├── user/key/            # Key 管理（全员可见）
│       │   ├── routing/             # 请求分流
│       │   │   ├── app-profile/     #   应用预设（admin）
│       │   │   └── traffic-plan/    #   应用方案（admin）
│       │   ├── upstream/            # 上游模型
│       │   │   ├── channel/         #   渠道管理（admin，含用量统计）
│       │   │   └── model/           #   模型管理（admin）
│       │   ├── control/             # 流量控制
│       │   │   ├── quota/           #   配额管理（admin，支持渠道级）
│       │   │   ├── ratelimit/       #   限流管理（admin）
│       │   │   └── user/            #   用户管理（admin，含 IP 白名单）
│       │   ├── data/                # 数据统计
│       │   │   ├── log/             #   请求日志（全员，user 仅看自己）
│       │   │   └── stats/           #   使用统计（全员，user 仅看自己）
│       │   └── settings/            # 系统设置（admin，含 IP 黑名单）
│       ├── service/api/             # API 请求封装
│       ├── typings/api/             # TypeScript 类型定义
│       ├── router/elegant/          # 自动生成的路由
│       └── layouts/                 # 布局组件（含版本号标识）
├── .github/workflows/
│   ├── docker.yml                   # Docker 镜像自动构建推送
│   └── release.yml                  # Release 自动发布
├── Dockerfile                       # 前后端单镜像多阶段构建
├── docker-compose.yml               # Docker Compose 编排
├── nginx.conf                       # Nginx 反向代理配置
├── start.sh                         # 容器启动脚本
└── deploy.sh                        # 原生一键部署脚本
```

---

## 权限模型

| 角色 | 说明 | 可访问内容 |
|------|------|-----------|
| **admin** | 管理员 | 全部功能：渠道管理、模型管理、用户管理、配额、限流、系统设置 |
| **user** | 普通用户 | 仪表盘（个人数据）、Key 管理（创建/删除自己的 API Key）、数据统计（仅自己的日志和统计） |

后端 `admin_auth_middleware` 强制检查 `role == "admin"`，前端路由通过 `roles: ['admin']` 元数据控制菜单可见性。

**API Key 权限：**
- 每个用户可创建多个 API Key
- 每个 Key 可设置独立的可访问模型列表（`enabled_models`）
- 代理层强制检查请求模型是否在 Key 的权限范围内

---

### GitHub Releases

推送新版本时会自动触发 GitHub Actions 创建 Release，并上传一键部署压缩包（含 `docker-compose.yml`、`.env.example`、`config.yaml`）。

---

## 技术架构

### 后端技术栈

| 组件 | 技术 |
|------|------|
| 语言 | Rust 2024 Edition |
| Web 框架 | [Axum](https://github.com/tokio-rs/axum) 0.8 |
| 数据库 | SQLite + [SQLx](https://github.com/launchbadge/sqlx) 0.8 |
| 异步运行时 | [Tokio](https://tokio.rs/) 1 |
| 序列化 | [Serde](https://serde.rs/) |
| 限流 | [Governor](https://github.com/antifuchs/governor) 0.6 |
| JWT | [jsonwebtoken](https://github.com/Keats/jsonwebtoken) 9 |
| 日志 | [tracing](https://github.com/tokio-rs/tracing) 0.1 + [tracing-appender](https://github.com/tokio-rs/tracing)（按天分割）|
| HTTP 客户端 | [reqwest](https://github.com/seanmonstar/reqwest) 0.12 |

### 前端技术栈

| 组件 | 技术 |
|------|------|
| 框架 | Vue 3 + TypeScript |
| UI 库 | [Naive UI](https://www.naiveui.com/) |
| 图表 | [ECharts](https://echarts.apache.org/) 6 |
| 模板 | [SoybeanAdmin](https://github.com/soybeanjs/soybean-admin) |
| 请求库 | [Alova](https://alova.js.org/) |
| 路由 | [elegant-router](https://github.com/soybeanjs/elegant-router) |

### 部署架构

```
用户请求 → Nginx (端口 80)
         ├── /admin/* → 后端 API (端口 8000)
         ├── /v1/*    → 后端代理 (端口 8000)
         └── /*       → Vue SPA 静态文件
```

---

## 主业务流程

```
┌──────────┐     API Key      ┌──────────────┐     渠道选择     ┌──────────────┐
│   用户    │ ──────────────→ │  CoRide-API   │ ──────────────→ │ 上游 AI 服务   │
│  (API Key) │ ←────────────── │   代理服务     │ ←────────────── │ OpenAI/       │
└──────────┘   JSON 响应      └──────────────┘   原始响应      │ Anthropic/等   │
                                                                    └──────────────┘
```

1. **用户发起请求**：通过 OpenAI 兼容接口 (`/v1/chat/completions` 等) 携带 API Key 发起请求
2. **认证鉴权**：`auth.rs` 验证 API Key，检查用户状态和速率限制
3. **模型权限**：检查请求模型是否在用户的 `enabled_models` 范围内
4. **配额检查**：先检查渠道配额，再检查用户配额
5. **渠道选择**：根据模型名称查找匹配的上游渠道（权重 + 轮询）
6. **应用伪装**：`resolve_app_profile_for_channel` 按渠道方案 → 全局方案 → 旧版 app_profile_id 优先级选择伪装配置
7. **代理转发**：转发至上游服务（支持流式 SSE 透传和非流式）
8. **响应处理**：解析 Token 用量，扣除配额，记录日志
9. **重试机制**：5xx 错误自动切换到下一个渠道重试

---

## 许可证

本项目采用 [Apache-2.0 许可证](LICENSE)。

---

## 致谢

- **[Axum](https://github.com/tokio-rs/axum)** —— Rust Web 框架
- **[Tokio](https://tokio.rs/)** —— Rust 异步运行时
- **[SQLx](https://github.com/launchbadge/sqlx)** —— Rust SQL 工具包
- **[Vue 3](https://vuejs.org/)** —— 前端框架
- **[Naive UI](https://www.naiveui.com/)** —— Vue 3 组件库
- **[ECharts](https://echarts.apache.org/)** —— 数据可视化
- **[SoybeanAdmin](https://github.com/soybeanjs/soybean-admin)** —— 管理后台模板
- **[Governor](https://github.com/antifuchs/governor)** —— 请求限流
- **[jsonwebtoken](https://github.com/Keats/jsonwebtoken)** —— JWT 令牌
- **[reqwest](https://github.com/seanmonstar/reqwest)** —— HTTP 客户端
- **[tracing](https://github.com/tokio-rs/tracing)** —— 结构化日志
- **[bcrypt](https://github.com/Keats/rust-bcrypt)** —— 密码哈希

---

## 联系方式

- **邮箱：** [cyfor@foxmail.com](mailto:cyfor@foxmail.com)
- **问题反馈：** 欢迎提 Issue 或 PR
