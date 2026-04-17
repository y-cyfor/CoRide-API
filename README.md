# CoRide-API

> 一个轻量级 AI API 代理服务 —— 多模型拼车中转

[![License](https://img.shields.io/badge/license-Apache--2.0-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)
[![Vue](https://img.shields.io/badge/vue-3-green.svg)](https://vuejs.org/)

---

## 项目简介

CoRide-API 是一个基于 Rust + Vue 3 构建的轻量级 AI API 代理管理工具。它允许多个用户共享一组上游 AI 服务渠道（如 OpenAI、Anthropic、阿里云通义千问、智谱 AI、Kimi 等），通过统一的 OpenAI 兼容接口对外提供服务。

**核心业务场景：**

- 管理员集中管理多个 AI 服务渠道的 API Key 和配置
- 多个用户通过各自的 API Key 接入代理服务
- 按用户进行配额限制（请求数 / Token 数）和速率控制
- 支持按用户绑定可用模型，实现精细化的权限管理
- 全量请求日志记录与统计分析，方便运维监控

**项目定位：** 纯无商业化功能的个人/小团队拼车工具，专注于简洁、高效、易部署。

---

## 关于 Vibe Coding

本项目完全通过 **Vibe Coding** 方式开发完成 —— 开发者通过自然语言描述需求，由 AI 编写代码实现。

| 项目 | 工具/模型 |
|------|-----------|
| 编程工具 | [Claude Code](https://github.com/anthropics/claude-code)、[OpenCode](https://github.com/opencode-ai/opencode) |
| 大语言模型 | Qwen 3.6-plus、小米 MiMo v2 Pro |

---

## 安装与使用

### 前置要求

- **Rust 1.80+**（后端开发）
- **Node.js 20+** + pnpm（前端开发）
- **Docker & Docker Compose**（可选）

### 方法一：从 Docker Hub 镜像部署

无需安装 Rust 或 Node.js，直接拉取预构建镜像运行。

**完整步骤：**

```bash
# 1. 拉取镜像
docker pull cyfor/coride-api:latest

# 2. 创建数据目录
mkdir -p /opt/coride/data /opt/coride/logs

# 3. 准备环境变量文件
cat > /opt/coride/.env << 'EOF'
CORIDE_ADMIN_USERNAME=admin
CORIDE_ADMIN_PASSWORD=你的密码
CORIDE_JWT_SECRET=你的随机密钥
CORIDE_LOG_LEVEL=info
EOF

# 4. 创建 docker-compose.yml
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

# 5. 启动
cd /opt/coride
docker compose up -d

# 6. 查看日志
docker logs -f coride-api
```

启动成功后访问 `http://服务器IP`，使用你在 `.env` 中设置的管理员账号登录。

### 方法二：Docker 本地构建部署

从源代码本地构建 Docker 镜像，适合需要自定义代码的场景。

```bash
# 1. 克隆项目
git clone https://github.com/y-cyfor/CoRide-API.git
cd CoRide-API

# 2. 准备 .env 文件
cp .env.example .env
# 编辑 .env 设置管理员密码和 JWT Secret

# 3. 构建并启动
docker compose up -d --build

# 4. 查看日志
docker compose logs -f
```

### 方法三：开发环境启动

#### 后端

```bash
cd backend

# 首次运行前确保配置存在
cp config/config.example.yaml config/config.yaml  # 按需修改配置

# 开发模式运行（自动编译 + 热重载）
cargo run
```

#### 前端

```bash
cd web

# 安装依赖
pnpm install

# 启动开发服务器
pnpm dev
```

### 方法四：本地构建

```bash
# 后端构建
cd backend
cargo build --release
./target/release/coride-api

# 前端构建
cd web
pnpm install
pnpm build
# 将 dist/ 部署到任意静态服务器即可
```

### 默认管理员账号

| 用户名 | 密码 |
|--------|------|
| admin  | admin123 |

> 请在生产环境中修改 `config/config.yaml` 中的管理员密码和 JWT Secret。

---

## 项目结构

```
CoRide-API/
├── backend/                  # Rust 后端（Axum 框架）
│   ├── src/
│   │   ├── main.rs           # 应用入口：路由注册、服务启动、后台任务
│   │   ├── lib.rs            # 库入口：AppState 定义
│   │   ├── config.rs         # 配置加载（YAML + 环境变量覆盖）
│   │   ├── state/            # 应用状态构建
│   │   ├── db/               # 数据库层
│   │   │   ├── mod.rs        # 连接池初始化 + 迁移执行
│   │   │   └── migrations/   # SQL 迁移文件（8 个）
│   │   ├── middleware/        # 中间件
│   │   │   ├── auth.rs       # 用户 API Key 认证
│   │   │   ├── admin_auth.rs # 管理员 JWT 认证
│   │   │   └── rate_limit.rs # 请求速率/并发限制
│   │   ├── router/           # 路由处理器
│   │   │   ├── proxy_routes.rs  # 代理请求处理 + 用户信息查询
│   │   │   └── admin_routes.rs  # 管理后台 CRUD + 统计
│   │   ├── service/          # 业务逻辑
│   │   │   ├── proxy.rs      # HTTP 代理（流式/非流式）
│   │   │   ├── openai.rs     # OpenAI 格式适配
│   │   │   ├── anthropic.rs  # Anthropic 格式适配
│   │   │   ├── quota.rs      # 配额检查与扣除
│   │   │   ├── health.rs     # 渠道健康检查
│   │   │   └── log.rs        # 日志记录
│   │   └── utils/
│   │       ├── jwt.rs        # JWT 令牌签发与验证
│   │       └── token_counter.rs  # Token 估算
│   └── config/config.yaml    # 配置文件
├── web/                      # Vue 3 前端管理后台
│   └── src/
│       ├── views/
│       │   ├── dashboard/    # 仪表盘（统计卡片 + 图表 + 实时表格）
│       │   ├── manage/
│       │   │   ├── user/     # 用户管理
│       │   │   ├── channel/  # 渠道管理
│       │   │   ├── model/    # 模型管理
│       │   │   ├── quota/    # 配额管理
│       │   │   ├── log/      # 请求日志
│       │   │   ├── stats/    # 统计分析
│       │   │   ├── settings/ # 系统设置
│       │   │   └── app-profile/ # 应用预设管理
│       │   └── ...
│       └── service/api/      # API 请求封装
├── Dockerfile.backend        # 后端多阶段构建
├── Dockerfile.frontend       # 前端多阶段构建
├── docker-compose.yml        # Docker Compose 编排
└── nginx.conf                # Nginx 反向代理配置
```

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
| 日志 | [tracing](https://github.com/tokio-rs/tracing) 0.1 |
| HTTP 客户端 | [reqwest](https://github.com/seanmonstar/reqwest) 0.12 |

### 前端技术栈

| 组件 | 技术 |
|------|------|
| 框架 | Vue 3 + TypeScript |
| UI 库 | [Naive UI](https://www.naiveui.com/) |
| 图表 | [ECharts](https://echarts.apache.org/) 6 |
| 模板 | [SoybeanAdmin](https://github.com/soybeanjs/soybean-admin) |
| 请求库 | [Alova](https://alova.js.org/) |

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
│   用户    │ ──────────────→ │  LiteProxy    │ ──────────────→ │ 上游 AI 服务   │
│  (API Key) │ ←────────────── │   代理服务     │ ←────────────── │ OpenAI/       │
└──────────┘   JSON 响应      └──────────────┘   原始响应      │ Anthropic/等   │
                                                                    └──────────────┘
```

1. **用户发起请求**：用户通过 OpenAI 兼容接口 (`/v1/chat/completions` 等) 携带 API Key 发起请求
2. **认证鉴权**：中间件验证 API Key，检查用户状态和速率限制
3. **渠道选择**：根据请求中的模型名称，查找匹配的上游渠道（按权重 + 轮询）
4. **配额检查**：先检查渠道配额，再检查用户配额
5. **代理转发**：将请求转发至上游服务（支持流式 SSE 透传和非流式两种模式）
6. **响应处理**：解析响应中的 Token 用量，扣除配额，记录日志
7. **重试机制**：如遇上游服务 5xx 错误，自动切换到下一个渠道重试

---

## 核心功能流程

### 用户管理

```
管理员创建用户 → 生成唯一 API Key → 绑定可用模型列表 → 创建配额
                                    ↓
用户自助查询: GET /v1/user/info → 返回配额用量 + 可用模型
```

- API Key 支持一键复制和掩码显示（前 8 位 + `****`）
- 用户可绑定特定模型，未绑定时可访问所有可用模型

### 渠道管理

```
管理员创建渠道 → 配置 BaseURL / API Keys / 权重 / 超时
              → 设置配额类型（请求数/Token数）+ 周期（小时/天/周/月）
              → 绑定应用预设（User-Agent 伪装）
```

- **健康检查**：后台每 5 分钟自动检测渠道可用性，连续 3 次失败自动禁用
- **配额进度**：列表显示彩色进度条，阈值：绿色 <70%、橙色 <90%、红色 >=90%
- **一键测试**：向渠道发送 `/models` 请求验证连通性

### 代理转发

```
请求到达 → 解析模型名 → 查找活跃渠道 → 检查配额
    ↓
流式请求？
├── 是 → SSE 流式透传（bytes_stream 直接转发）
└── 否 → 完整响应模式（解析 Token 用量，格式化返回）
    ↓
扣除配额 → 记录日志 → 返回响应
```

- 支持 OpenAI 兼容接口 (`/v1/chat/completions`, `/v1/completions`) 和 Anthropic Messages 接口 (`/v1/messages`)
- 流式响应通过 `reqwest::bytes_stream()` 直接透传，无需等待完整响应
- 渠道失败自动重试（可配置重试次数）

### 配额管理

```
用户/渠道配额 → 类型（requests / tokens）
            → 周期（hourly / daily / weekly / monthly / permanent）
            → 自动重置：周期到期时 quota_used 归零
```

- 配额检查顺序：先渠道后用户，避免渠道无配额时仍消耗用户配额
- 并发安全：使用 CAS 原子操作防止竞态条件

### 日志与统计

```
每次请求 → 记录 API Key / 渠道 / 模型 / Token / 状态码 / 耗时
                                              ↓
仪表盘：统计卡片 + 7 天趋势折线图 + 渠道饼图 + Token 堆叠柱状图
统计页：按用户/渠道/模型筛选 + 自定义时间范围
日志页：模型/状态码/日期范围筛选 + 请求体/响应体详情
```

- 请求/响应体日志记录可通过配置开关控制
- 日志自动清理：默认保留 30 天
- 支持 CSV 导出

### 应用预设伪装

```
创建应用预设 → 名称 + 标识符（小写字母+连字符校验）
            → User-Agent + Extra Headers
            → 系统预设不可删除，用户预设可编辑
```

- 为渠道设置请求头伪装，模拟不同客户端行为
- 系统内置常用预设，不可删除

### 配额预警

```
后台检测 → 配额使用 >= 80% 触发 info 预警
         → >= 90% 触发 warning 预警
         → >= 100% 触发 critical 告警
         → 仪表盘顶部 Alert 卡片展示
```

---

## 许可证

本项目采用 [Apache-2.0 许可证](LICENSE)。

```
Copyright 2026 CoRide-API Contributors

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

---

## 致谢

本项目基于以下开源项目和框架构建：

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
