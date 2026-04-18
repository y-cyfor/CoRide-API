# CoRide-API 产品需求文档 (PRD)

> 最后更新: 2026-04-17

---

## 一、产品概述

### 1.1 产品定位

CoRide-API 是一个轻量级 AI API 中转管理平台，面向个人开发者和小团队，提供多供应商 AI 服务渠道的集中管理和用户级权限分发。

### 1.2 核心业务价值

- **渠道聚合**：统一管理多个 AI 供应商（OpenAI、Anthropic、阿里云、智谱、Kimi、小米、MiniMax 等）
- **用户分发**：多用户通过独立 API Key 接入，互不干扰
- **成本控制**：配额限制（请求数/Token数）+ 速率限制，防止超额消耗
- **合规伪装**：分时段、按比例将请求伪装为不同应用（UA + 请求头），应对供应商风控
- **运维透明**：全量请求日志 + 实时统计 + 渠道健康监控

### 1.3 目标用户

| 用户角色 | 描述 |
|----------|------|
| **管理员** | 拥有全部权限，负责渠道配置、用户管理、系统运维 |
| **普通用户** | 通过 API Key 调用代理接口，管理自己的 Key，查看个人使用统计 |

---

## 二、业务逻辑

### 2.1 用户管理

```
管理员登录 → 用户管理 → 创建用户 → 系统生成 API Key
                                    ↓
                          绑定可访问模型（可选）
                                    ↓
                          创建配额（周期 + 上限）
```

- 用户只能查看和管理自己的 API Key（Key 管理页面）
- 每个 API Key 可设置独立的可访问模型列表
- 未设置时继承用户的默认模型权限
- 管理员可查看所有用户的 Key 列表，但不可创建

### 2.2 代理请求流程

```
客户端请求 → /v1/chat/completions (携带 API Key)
    │
    ▼
┌─────────────────────────────────────────────────┐
│ 1. API Key 认证 (auth.rs)                        │
│    - 查找 user_keys 表，获取 user_id             │
│    - 检查用户状态是否 active                     │
│    - 获取该 Key 的 enabled_models               │
│    - 若 Key 无独立设置，回退到 users.enabled_models │
└────────────┬────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────┐
│ 2. 全局速率限制 (rate_limit.rs)                  │
│    - Governor QPS 令牌桶                         │
│    - 并发数原子计数器                            │
└────────────┬────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────┐
│ 3. 模型权限检查                                  │
│    - 请求模型是否在 enabled_models 范围内        │
│    - 空列表 = 全部可访问                         │
│    - 不在范围内 → 返回 403                       │
└────────────┬────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────┐
│ 4. 模型路由 → 渠道选择                           │
│    - 根据请求模型名查找匹配的渠道                │
│    - 按权重加权轮询选择渠道                      │
│    - 检查渠道状态（active/disabled）             │
└────────────┬────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────┐
│ 5. 应用伪装选择 (resolve_app_profile_for_channel) │
│    - 渠道专属流量方案 → 全局流量方案 → 旧版 app_profile │
│    - 根据当前 UTC 小时匹配时段槽                 │
│    - 加权随机选择 app_profile（混合熵种子）       │
│    - 设置 User-Agent + Extra Headers             │
└────────────┬────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────┐
│ 6. 配额检查                                      │
│    - 渠道配额（若设置）                          │
│    - 用户配额                                    │
│    - 周期自动重置（hourly/daily/weekly/monthly） │
│    - 超限 → 返回 402                             │
└────────────┬────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────┐
│ 7. 代理转发                                      │
│    - OpenAI 接口: /v1/chat/completions, /v1/completions │
│    - Anthropic 接口: /v1/messages                │
│    - 流式: SSE bytes_stream 透传                 │
│    - 非流式: 完整响应后解析 Token 用量            │
└────────────┬────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────┐
│ 8. 响应处理                                      │
│    - 扣除渠道/用户配额                           │
│    - 记录请求日志（模型/Token/状态码/耗时）       │
│    - 5xx 自动重试（按 retry_count）              │
└─────────────────────────────────────────────────┘
```

### 2.3 流量分发（应用方案）

```
管理员创建全局方案
    │
    ├── 时段 1: 00:00-08:00 → Claude Code (70%) + OpenCode (30%)
    ├── 时段 2: 08:00-18:00 → Claude Code (50%) + Generic (50%)
    └── 时段 3: 18:00-24:00 → OpenCode (60%) + Claude Code (40%)
    
管理员创建渠道专属方案（可选，优先级高于全局方案）
    │
    └── 渠道 A: 独立时段和权重配置
    
代理请求到达时：
    1. 检查当前渠道是否有专属方案 → 匹配当前小时 → 加权随机选应用
    2. 回退到全局方案 → 匹配当前小时 → 加权随机选应用
    3. 回退到渠道绑定的 app_profile_id（旧版兼容）
    4. 无伪装
```

### 2.4 配额管理

```
用户配额：
    ├── 类型：requests（次数）或 tokens（Token数），二选一
    ├── 周期：hourly / daily / weekly / monthly / permanent
    └── 自动重置：周期结束时 used 归零

渠道配额：
    ├── 类型：requests / tokens / null（不限）
    ├── 周期：hourly / daily / weekly / monthly / permanent
    └── 用途：防止单个渠道超额消耗
```

### 2.5 日志与统计

```
每次代理请求自动记录：
    API Key → 用户映射、渠道 ID、模型名、端点、状态码、
    Token 用量（prompt/completion/total）、耗时、错误信息

仪表盘（实时）：
    ├── 总请求数、今日请求、活跃用户、成功/失败次数、P95 耗时、总 Token
    ├── 近 N 天请求趋势（折线图）
    ├── 渠道使用分布（饼图）
    ├── Token 消耗分布（堆叠柱状图：输入/输出）
    └── 最新 10 条请求（实时表格）

统计页（可筛选）：
    ├── 按渠道/模型/天数筛选
    ├── 用户调用排名（TOP 10 柱状图）
    ├── 模型调用分布（TOP 10 饼图）
    └── 支持导出 CSV

日志页（可筛选）：
    ├── 按模型/状态码/日期范围筛选
    ├── 日期范围服务端过滤
    ├── 请求详情抽屉（请求体/响应体 JSON）
    └── user 用户只能查看自己的日志
```

### 2.6 权限控制

```
菜单权限：
    ├── admin 可见：仪表盘、请求分流（应用预设/应用方案）、上游模型（渠道/模型）、
    │              流量控制（配额/限流/用户）、数据统计（日志/统计）、系统设置
    └── 全员可见：Key 管理、数据统计

数据权限：
    ├── user 用户：只能查看自己的日志、统计、Key
    └── admin 用户：查看全部数据，可筛选特定用户

后端强制：
    ├── admin_auth_middleware 检查 role == "admin"
    ├── auth.rs 检查 API Key 对应的用户状态和模型权限
    └── 前端路由守卫仅做菜单可见性控制
```

---

## 三、技术架构

### 3.1 后端架构

```
backend/
├── main.rs              # 路由注册 → 后台任务（日志清理 + 健康检查）→ 优雅关闭
├── config.rs            # YAML 配置 + 环境变量覆盖（CORIDE_ 优先，LP_ 兼容）
├── middleware/
│   ├── auth.rs          # 用户 API Key 认证 + 模型权限检查
│   ├── admin_auth.rs    # 管理员 JWT 认证 + 角色检查
│   └── rate_limit.rs    # 全局 QPS + 并发限制（Governor + AtomicU32）
├── router/
│   ├── proxy_routes.rs  # /v1/* 代理路由（chat/completions, completions, messages, models, user/info）
│   └── admin_routes.rs  # /admin/* + /user/* 管理路由（60+ 接口）
├── service/
│   ├── proxy.rs         # HTTP 代理核心（流式/非流式 + 重试 + 应用伪装）
│   ├── openai.rs        # OpenAI 格式适配
│   ├── anthropic.rs     # Anthropic 格式适配
│   ├── quota.rs         # 配额检查/扣除/周期重置
│   ├── health.rs        # 渠道健康检查（5 分钟轮询，连续 3 次失败自动禁用）
│   └── log.rs           # 请求日志写入
├── db/
│   ├── mod.rs           # SQLite 连接池 + 迁移执行
│   ├── migrations/      # 10 个 SQL 迁移文件
│   └── models.rs        # 所有结构体 + CRUD 函数（含 resolve_app_profile_for_channel）
└── utils/
    ├── jwt.rs           # JWT 签发/验证
    └── token_counter.rs # Token 估算（字符计数启发式）
```

### 3.2 数据库模型

```
users ──────────────┐
                    ├── quotas ────────────── 用户配额
channels ───────────┼── channel_models ────── 渠道模型映射
 ├── app_profiles ──┤                        （models 表含 channel_id）
 ├── traffic_plans ─┤
 │   └── slots ─────┤                       时段 + 权重分流
user_keys ──────────┤                       用户多 Key 管理
ratelimit_configs ──┤                       多级限流
request_logs ───────┘                       请求审计
```

| 表 | 说明 |
|---|------|
| `users` | 用户账户（用户名、密码哈希、role、enabled_models）|
| `channels` | 上游渠道（BaseURL、API Keys 轮询、权重、配额）|
| `models` | 模型映射（source_name → proxy_name，按渠道分组）|
| `app_profiles` | 应用预设（User-Agent + Extra Headers 模板）|
| `traffic_plans` | 流量方案（NULL channel_id = 全局方案）|
| `traffic_plan_slots` | 方案时段槽（start_hour/end_hour/app_profile/weight）|
| `quotas` | 配额（用户/渠道级，周期自动重置）|
| `ratelimit_configs` | 限流规则（global/channel/user 级 QPS/并发）|
| `user_keys` | 用户 API Key（独立 enabled_models、状态）|
| `request_logs` | 请求日志（自动清理，保留 30 天）|

### 3.3 前端架构

```
web/src/
├── views/                 # 页面视图（按业务域分组）
├── service/api/           # Alova 请求封装（按模块分文件）
├── typings/api/           # TypeScript 类型定义
├── router/elegant/        # 自动生成的路由（elegant-router 插件）
├── store/modules/         # Pinia 状态管理（auth, route, theme, app）
├── layouts/               # 布局组件（base/blank + 菜单/头部/标签栏）
├── hooks/common/          # 组合式函数（router, echarts, table, version）
└── locales/langs/         # i18n 多语言（zh-cn, en-us）
```

### 3.4 部署架构

| 部署方式 | 镜像/构建 | 端口 | 数据持久化 |
|----------|-----------|------|-----------|
| Docker Hub | `cyfor/coride-api:latest` | 80 | volume 映射 |
| 本地构建 | `docker compose up -d --build` | 80 | volume 映射 |
| 原生部署 | `./deploy.sh` | 自定义 | `./data/` |
| 开发环境 | `cargo run` + `pnpm dev` | 8000/9527 | 本地文件 |

**Docker 单镜像设计：**
- 多阶段构建：Rust 编译 + Node 编译 + Debian 运行
- 内部：nginx（80 端口）反向代理到后端（8000 端口）
- 启动脚本：同时启动后端二进制和 nginx
- 数据卷：`/app/data`（数据库）、`/app/log`（日志）

### 3.5 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `CORIDE_PORT` | 服务端口 | `8000` |
| `CORIDE_DB_PATH` | 数据库路径 | `./data/coride.db` |
| `CORIDE_ADMIN_USERNAME` | 初始管理员用户名 | `admin` |
| `CORIDE_ADMIN_PASSWORD` | 初始管理员密码 | `admin123` |
| `CORIDE_JWT_SECRET` | JWT 签名密钥 | `change-me-to-random-string` |
| `CORIDE_LOG_LEVEL` | 日志级别 | `info` |

兼容旧前缀 `LP_`（回退匹配）。

---

## 四、功能清单

| 模块 | 功能 | 可见性 | 状态 |
|------|------|--------|------|
| 仪表盘 | 统计卡片 + 趋势图 + 饼图 + 柱状图 | 全员（user 看个人） | ✅ |
| Key 管理 | 创建/编辑/删除 API Key，设置模型权限 | 全员（仅自己的 Key） | ✅ |
| 请求分流 | 应用预设管理（UA + 请求头） | admin | ✅ |
| 请求分流 | 应用方案管理（时段 + 权重分流） | admin | ✅ |
| 上游模型 | 渠道管理（创建/测试/配额/伪装/用量统计） | admin | ✅ |
| 上游模型 | 模型管理（树形列表 + 映射） | admin | ✅ |
| 流量控制 | 配额管理（创建/编辑/周期/渠道级） | admin | ✅ |
| 流量控制 | 限流管理（QPS/并发/动作） | admin | ✅ |
| 流量控制 | 用户管理（创建/编辑/重置 Key/IP 白名单） | admin | ✅ |
| 数据统计 | 请求日志（筛选 + 详情 + 导出） | 全员（user 仅自己的） | ✅ |
| 数据统计 | 使用统计（筛选 + 图表） | 全员（user 仅自己的） | ✅ |
| 系统设置 | 服务器/数据库/日志/代理/限流/IP 黑名单配置 | admin | ✅ |
| 版本号 | 侧边栏底部版本号 + 自动检测更新 | 全员 | ✅ |

---

## 五、非功能需求

| 需求 | 说明 |
|------|------|
| 性能 | 单实例支持 100+ QPS，流式响应无内存拷贝 |
| 安全 | 密码 bcrypt 哈希、JWT 认证、后端角色强制检查 |
| 可靠 | 渠道失败自动重试、健康检查自动禁用异常渠道 |
| 可维护 | 按天分割的日志文件、30 天自动清理 |
| 可部署 | Docker 单镜像、一键脚本、环境变量覆盖 |
