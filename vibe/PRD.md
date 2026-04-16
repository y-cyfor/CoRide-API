# CoRide-API - 轻量级大模型API中转服务 - 详细开发方案

## 一、项目概述

### 1.1 项目信息

| 项目 | 内容 |
|------|------|
| 名称 | CoRide-API |
| 定位 | 轻量级、高性能、开箱即用的 LLM API 中转服务 |
| 目标用户 | 个人开发者、小团队、朋友间共享 |
| 核心价值 | 轻量（单二进制 < 20MB）、美观、功能齐全 |

### 1.2 核心功能清单

- ✅ 多供应商接入（OpenAI、Anthropic，可扩展）
- ✅ 接口兼容（OpenAI + Anthropic 标准接口）
- ✅ 应用伪装预设（Claude Code / OpenCode / OpenClaw 等，可切换）
- ✅ 多级限流（全局/渠道/用户）
- ✅ API Key 鉴权 + 模型映射
- ✅ 配额系统（次数/Token **二选一**，自定义周期）
- ✅ **渠道总限额**（Token/次数，达到后拒绝请求）
- ✅ SQLite 嵌入式存储
- ✅ Web 管理面板（SoybeanAdmin）
- ✅ 登录认证（无注册，管理员手动创建）
- ✅ 调用统计 + 日志管理
- ✅ Docker 一键部署
- ✅ **预设国内供应商配置**（阿里云/智谱/Kimi/小米/Minimax 及其 codingplan）

---

## 二、技术栈

### 2.1 后端

| 组件 | 选型 | 版本 |
|------|------|------|
| 语言 | **Rust** | 2024 Edition |
| Web 框架 | **Axum** | 0.8+ |
| HTTP 客户端 | **Reqwest** | 0.12+ |
| ORM/DB | **SQLx** + SQLite | 0.8+ |
| 限流 | **governor** | 0.6+ |
| 序列化 | **serde** + **serde_json** | 1.0+ |
| 配置 | **serde_yaml** | 0.9+ |
| 日志 | **tracing** + **tracing-subscriber** | 0.1+ |
| 密码哈希 | **bcrypt** | 0.16+ |
| JWT | **jsonwebtoken** | 9+ |
| 异步运行时 | **Tokio** | 1+ |

### 2.2 前端（管理面板）

| 组件 | 选型 | 版本 |
|------|------|------|
| UI 框架 | **Soybean Admin** | 最新版 |
| 基础框架 | Vue 3 + TypeScript | 3.4+ / 5.3+ |
| UI 组件库 | Naive UI | 2.38+ |
| 状态管理 | Pinia | 2.1+ |
| 构建工具 | Vite | 6+ |
| 路由 | Vue Router | 4+ |
| HTTP 客户端 | Axios | 1.7+ |
| 图表 | ECharts | 5.5+ |

### 2.3 部署

| 组件 | 说明 |
|------|------|
| Docker | 多阶段构建，镜像 < 50MB |
| Docker Compose | 一键启动（后端 + 前端静态资源） |

---

## 三、项目结构

```
liteproxy/
├── backend/                          # Rust 后端
│   ├── Cargo.toml
│   ├── .env.example                  # 环境变量示例
│   ├── config/
│   │   └── config.yaml               # 应用配置
│   ├── src/
│   │   ├── main.rs                   # 入口
│   │   ├── lib.rs                    # 库根
│   │   ├── config.rs                 # 配置加载与解析
│   │   ├── db/
│   │   │   ├── mod.rs                # 数据库初始化
│   │   │   ├── migrations/           # 数据库迁移（SQLx 离线模式）
│   │   │   │   ├── 001_init.sql
│   │   │   │   ├── 002_users.sql
│   │   │   │   ├── 003_channels.sql
│   │   │   │   ├── 004_models.sql
│   │   │   │   ├── 005_quotas.sql
│   │   │   │   ├── 006_app_profiles.sql    # 应用伪装预设表
│   │   │   │   └── 007_channel_quotas.sql  # 渠道配额字段 + app_profile_id
│   │   │   └── models.rs             # 数据库实体
│   │   ├── middleware/
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs               # API Key 鉴权中间件
│   │   │   ├── rate_limit.rs         # 限流中间件
│   │   │   └── request_header.rs     # 请求头修改中间件
│   │   ├── service/
│   │   │   ├── mod.rs
│   │   │   ├── proxy.rs              # 代理转发核心逻辑
│   │   │   ├── openai.rs             # OpenAI 接口适配
│   │   │   ├── anthropic.rs          # Anthropic 接口适配
│   │   │   ├── quota.rs              # 配额计算与扣减
│   │   │   └── log.rs                # 日志记录
│   │   ├── router/
│   │   │   ├── mod.rs
│   │   │   ├── proxy_routes.rs       # 中转路由
│   │   │   └── admin_routes.rs       # 管理面板 API
│   │   ├── state/
│   │   │   ├── mod.rs
│   │   │   └── app_state.rs          # 全局共享状态（限流器、DB连接等）
│   │   └── utils/
│   │       ├── mod.rs
│   │       └── token_counter.rs      # Token 估算工具
│   └── tests/
│       └── proxy_tests.rs
│
├── frontend/                         # 前端管理面板
│   ├── package.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── index.html
│   └── src/
│       ├── main.ts
│       ├── App.vue
│       ├── router/
│       │   └── index.ts
│       ├── store/
│       │   ├── index.ts
│       │   └── modules/
│       │       ├── auth.store.ts
│       │       ├── user.store.ts
│       │       └── config.store.ts
│       ├── api/
│       │   ├── index.ts
│       │   ├── auth.api.ts
│       │   ├── user.api.ts
│       │   ├── channel.api.ts
│       │   ├── model.api.ts
│       │   ├── app_profile.api.ts  # 应用伪装预设 API
│       │   ├── quota.api.ts
│       │   ├── stat.api.ts
│       │   ├── log.api.ts
│       │   └── types.ts              # 所有 API 类型定义
│       ├── views/
│       │   ├── login/
│       │   │   └── Login.vue
│       │   ├── layout/
│       │   │   └── DefaultLayout.vue
│       │   ├── dashboard/
│       │   │   └── Dashboard.vue
│       │   ├── system/
│       │   │   └── SystemSettings.vue
│       │   ├── channel/
│       │   │   ├── ChannelList.vue
│       │   │   └── ChannelForm.vue
│       │   ├── model/
│       │   │   ├── ModelList.vue
│       │   │   └── ModelForm.vue
│       │   ├── user/
│       │   │   ├── UserList.vue
│       │   │   └── UserForm.vue
│       │   ├── quota/
│       │   │   └── QuotaManage.vue
│       │   ├── ratelimit/
│       │   │   └── RateLimitConfig.vue
│       │   ├── stat/
│       │   │   └── UsageStats.vue
│       │   └── log/
│       │       └── LogViewer.vue
│       └── components/
│           └── ...                   # 共用组件
│
├── deploy/
│   ├── Dockerfile.backend
│   ├── Dockerfile.frontend
│   └── docker-compose.yml
│
└── README.md
```

---

## 四、数据库设计

### 4.1 ER 关系

```
users ──────────────┐
                    ├── user_quotas ──┐
                    │                 │
channels ───────────┼── channel_models┼── models
 │                  │                 │
 ├─ app_profiles ───┤                 │
 │                  │                 │
 ├─ traffic_plans ──┤                 │
 │   └─ slots ──────┤                 │
 │                  │                 │
rate_limit_configs ─┤                 │
request_logs ───────┘                 │
                                    └─ quotas
```

### 4.2 表结构

#### users 表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | 主键 |
| username | VARCHAR(64) | UNIQUE NOT NULL | 登录用户名 |
| password_hash | VARCHAR(128) | NOT NULL | bcrypt 哈希 |
| role | VARCHAR(16) | NOT NULL DEFAULT 'user' | 'admin' 或 'user' |
| api_key | VARCHAR(128) | UNIQUE NOT NULL | 中转调用 Key |
| status | VARCHAR(16) | NOT NULL DEFAULT 'active' | 'active' / 'disabled' |
| enabled_models | TEXT | NULL | JSON 数组，绑定的模型列表 |
| note | TEXT | NULL | 备注 |
| created_at | DATETIME | NOT NULL DEFAULT CURRENT_TIMESTAMP | 创建时间 |
| updated_at | DATETIME | NOT NULL DEFAULT CURRENT_TIMESTAMP | 更新时间 |

#### channels 表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | 主键 |
| name | VARCHAR(64) | UNIQUE NOT NULL | 渠道名称 |
| type | VARCHAR(32) | NOT NULL | 'openai' / 'anthropic' |
| base_url | VARCHAR(512) | NOT NULL | API 基础地址 |
| api_keys | TEXT | NOT NULL | JSON 数组，多个 Key 轮询 |
| custom_headers | TEXT | NULL | JSON 对象，自定义请求头 |
| status | VARCHAR(16) | NOT NULL DEFAULT 'active' | 'active' / 'disabled' |
| weight | INTEGER | NOT NULL DEFAULT 1 | 权重（负载均衡用） |
| timeout | INTEGER | NOT NULL DEFAULT 300000 | ms |
| retry_count | INTEGER | NOT NULL DEFAULT 1 | 重试次数 |
| **quota_type** | VARCHAR(16) | NULL | **'requests' / 'tokens' / NULL（不限）** |
| **quota_limit** | BIGINT | NULL | **总上限（与 quota_type 配合使用）** |
| **quota_used** | BIGINT | NOT NULL DEFAULT 0 | **已使用量** |
| **quota_cycle** | VARCHAR(16) | NULL | **'hourly' / 'daily' / 'weekly' / 'monthly' / NULL（永久）** |
| **quota_period_start** | DATETIME | NULL | **当前周期开始时间** |
| **quota_period_end** | DATETIME | NULL | **当前周期结束时间** |
| **app_profile_id** | INTEGER | NULL | **关联的应用伪装预设 ID（NULL = 使用自定义 headers）** |
| created_at | DATETIME | NOT NULL DEFAULT CURRENT_TIMESTAMP |
| updated_at | DATETIME | NOT NULL DEFAULT CURRENT_TIMESTAMP |

#### app_profiles 表（应用伪装预设）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | 主键 |
| name | VARCHAR(64) | UNIQUE NOT NULL | 预设名称（如 "Claude Code"） |
| identifier | VARCHAR(64) | UNIQUE NOT NULL | 唯一标识符（如 "claude-code"） |
| user_agent | VARCHAR(512) | NOT NULL | User-Agent 字符串 |
| extra_headers | TEXT | NULL | JSON 对象，额外请求头（如 `{"anthropic-version": "2023-06-01"}`） |
| description | TEXT | NULL | 描述说明 |
| enabled | BOOLEAN | NOT NULL DEFAULT true | 是否启用 |
| is_system | BOOLEAN | NOT NULL DEFAULT false | 是否系统内置（系统预设不可删除） |
| created_at | DATETIME | NOT NULL DEFAULT CURRENT_TIMESTAMP |

**系统内置预设列表**（`is_system = true`）：

| 预设名称 | identifier | 说明 |
|----------|-----------|------|
| Claude Code | `claude-code` | Claude Code CLI 的请求头特征 |
| OpenCode | `opencode` | OpenCode CLI 的请求头特征 |
| OpenClaw | `openclaw` | OpenClaw CLI 的请求头特征 |
| Generic OpenAI | `generic-openai` | 标准 OpenAI SDK 默认请求头 |
| Generic Anthropic | `generic-anthropic` | 标准 Anthropic SDK 默认请求头 |

> 管理员可自定义添加更多应用预设。

#### traffic_plans 表（流量分流方案）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | 主键 |
| channel_id | INTEGER | NULL | 关联渠道 ID，**NULL = 全局方案** |
| created_at | DATETIME | NOT NULL DEFAULT CURRENT_TIMESTAMP | 创建时间 |

> 每个渠道最多有一个专属方案，未配置专属方案的渠道继承全局方案。

#### traffic_plan_slots 表（方案时段槽）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | 主键 |
| plan_id | INTEGER | NOT NULL | 关联方案 ID |
| start_hour | INTEGER | NOT NULL CHECK(0-23) | 时段开始小时 |
| end_hour | INTEGER | NOT NULL CHECK(0-24) | 时段结束小时（24=午夜） |
| app_profile_id | INTEGER | NOT NULL | 关联应用预设 ID |
| weight | INTEGER | NOT NULL DEFAULT 100 CHECK(>0) | 权重（相对比例，不要求和为100） |

**业务逻辑**：
1. 代理请求到达时，根据当前 UTC 小时查找匹配的时段槽（`start_hour <= current_hour < end_hour`）
2. 若匹配到多个时段槽（不推荐），按权重加权随机选择
3. 优先级：**渠道专属方案 → 全局方案 → 旧版 channel.app_profile_id → 无伪装**
4. 跨午夜时段（如 23:00-02:00）需拆分为两个槽（23:00-24:00 + 0:00-2:00）

#### models 表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | 主键 |
| channel_id | INTEGER | NOT NULL | 关联渠道 ID |
| source_name | VARCHAR(128) | NOT NULL | 原始模型名（如 gpt-4o） |
| proxy_name | VARCHAR(128) | NOT NULL | 中转模型名（如 my-gpt4） |
| enabled | BOOLEAN | NOT NULL DEFAULT true | 是否启用 |
| is_default | BOOLEAN | NOT NULL DEFAULT false | 是否默认渠道 |
| created_at | DATETIME | NOT NULL DEFAULT CURRENT_TIMESTAMP |

#### ratelimit_configs 表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | 主键 |
| target_type | VARCHAR(32) | NOT NULL | 'global' / 'channel' / 'user' |
| target_id | INTEGER | NULL | 目标 ID（NULL = global） |
| qps | INTEGER | NOT NULL DEFAULT 0 | QPS 限制（0 = 不限） |
| concurrency | INTEGER | NOT NULL DEFAULT 0 | 并发限制（0 = 不限） |
| action | VARCHAR(16) | NOT NULL DEFAULT 'reject' | 'reject' / 'queue' |
| created_at | DATETIME | NOT NULL DEFAULT CURRENT_TIMESTAMP |

#### quotas 表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | 主键 |
| user_id | INTEGER | NOT NULL | 关联用户 |
| quota_type | VARCHAR(16) | NOT NULL | **'requests'（次数）/ 'tokens'（Token），每个用户二选一** |
| total_limit | BIGINT | NOT NULL | 总上限 |
| used | BIGINT | NOT NULL DEFAULT 0 | 已使用 |
| cycle | VARCHAR(16) | NOT NULL DEFAULT 'permanent' | **'hourly' / 'daily' / 'weekly' / 'monthly' / 'permanent'（永久）** |
| period_start | DATETIME | NULL | 当前周期开始时间 |
| period_end | DATETIME | NULL | 当前周期结束时间 |
| enabled | BOOLEAN | NOT NULL DEFAULT true | 是否启用 |
| created_at | DATETIME | NOT NULL DEFAULT CURRENT_TIMESTAMP |

> **二选一说明**：用户配额只设置一种类型（次数 **或** Token），不能同时设置两种。例如阿里云百炼 codingplan 按调用次数限制，则配额类型为 `requests`；如果供应商按 Token 消耗计费，则配额类型为 `tokens`。周期可选：时/日/周/月/永久。

#### request_logs 表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | BIGINT | PRIMARY KEY AUTOINCREMENT | 主键 |
| user_api_key | VARCHAR(128) | NOT NULL | 用户 Key |
| channel_id | INTEGER | NULL | 使用的渠道 |
| model | VARCHAR(128) | NOT NULL | 模型名 |
| endpoint | VARCHAR(256) | NOT NULL | 请求端点 |
| status_code | INTEGER | NOT NULL | 响应状态码 |
| prompt_tokens | INTEGER | NOT NULL DEFAULT 0 | 输入 Token |
| completion_tokens | INTEGER | NOT NULL DEFAULT 0 | 输出 Token |
| total_tokens | INTEGER | NOT NULL DEFAULT 0 | 总 Token |
| elapsed_ms | INTEGER | NOT NULL | 耗时(ms) |
| request_body | TEXT | NULL | 请求体摘要 |
| response_body | TEXT | NULL | 响应体摘要 |
| error_message | TEXT | NULL | 错误信息 |
| created_at | DATETIME | NOT NULL DEFAULT CURRENT_TIMESTAMP | 创建时间 |

---

## 五、API 接口设计

### 5.1 中转接口（用户调用，需 Bearer Token 鉴权）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/v1/chat/completions` | OpenAI 兼容对话接口 |
| POST | `/v1/completions` | OpenAI 兼容补全接口 |
| GET | `/v1/models` | 列出可用模型 |
| POST | `/v1/messages` | Anthropic 兼容接口 |

**鉴权方式**：`Authorization: Bearer <USER_API_KEY>`
**请求头伪装**：中间件根据渠道配置替换 `User-Agent` 等请求头
**限流**：依次检查 全局限流 → 用户限流 → 渠道限流

### 5.2 管理面板 API（需管理员 JWT 鉴权）

#### 认证

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/admin/auth/login` | 管理员登录，返回 JWT |
| GET | `/admin/auth/me` | 获取当前用户信息 |

#### 系统设置

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/admin/system` | 获取系统设置 |
| PUT | `/admin/system` | 更新系统设置 |

#### 应用伪装

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/admin/app-profiles` | 获取应用伪装预设列表 |
| POST | `/admin/app-profiles` | 创建自定义预设 |
| PUT | `/admin/app-profiles/:id` | 更新预设 |
| DELETE | `/admin/app-profiles/:id` | 删除预设（仅限非系统内置） |

#### 渠道管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/admin/channels` | 获取渠道列表 |
| POST | `/admin/channels` | 创建渠道 |
| GET | `/admin/channels/:id` | 获取渠道详情 |
| PUT | `/admin/channels/:id` | 更新渠道 |
| DELETE | `/admin/channels/:id` | 删除渠道 |
| POST | `/admin/channels/:id/test` | 测试渠道连通性 |

#### 模型管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/admin/models` | 获取模型列表 |
| POST | `/admin/models` | 创建模型映射 |
| PUT | `/admin/models/:id` | 更新模型 |
| DELETE | `/admin/models/:id` | 删除模型 |

#### 用户管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/admin/users` | 获取用户列表 |
| POST | `/admin/users` | 创建用户 |
| PUT | `/admin/users/:id` | 更新用户 |
| DELETE | `/admin/users/:id` | 删除用户 |
| POST | `/admin/users/:id/reset-key` | 重置 API Key |

#### 配额管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/admin/quotas` | 获取配额列表（?user_id=） |
| POST | `/admin/quotas` | 创建配额 |
| PUT | `/admin/quotas/:id` | 更新配额 |
| DELETE | `/admin/quotas/:id` | 删除配额 |

#### 限流配置

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/admin/ratelimits` | 获取所有限流配置 |
| POST | `/admin/ratelimits` | 创建限流规则 |
| PUT | `/admin/ratelimits/:id` | 更新限流规则 |
| DELETE | `/admin/ratelimits/:id` | 删除限流规则 |

#### 统计

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/admin/stats/dashboard` | 仪表盘概览数据 |
| GET | `/admin/stats/usage` | 调用统计（?start=&end=&user_id=&model=&channel_id=） |
| GET | `/admin/stats/export` | 导出 CSV |

#### 日志

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/admin/logs` | 获取日志列表（?page=&size=&user_id=&model=&status=） |
| GET | `/admin/logs/:id` | 获取日志详情 |
| DELETE | `/admin/logs` | 清理日志（?before=） |

---

## 六、核心流程设计

### 6.1 请求代理流程

```
客户端请求
    │
    ▼
┌─────────────────────────────────┐
│  1. Auth 中间件                  │
│     - 验证 Authorization: Bearer │
│     - 查询用户状态（是否禁用）    │
│     - 绑定 user_id 到 context     │
└──────────────┬──────────────────┘
               │
               ▼
┌─────────────────────────────────┐
│  2. 全局限流中间件               │
│     - 检查全局 QPS / 并发        │
│     - 触发 → 返回 429            │
└──────────────┬──────────────────┘
               │
               ▼
┌─────────────────────────────────┐
│  3. 用户限流中间件               │
│     - 检查该用户 QPS / 并发      │
│     - 触发 → 返回 429            │
└──────────────┬──────────────────┘
               │
               ▼
┌─────────────────────────────────┐
│  4. 配额检查                     │
│     - 检查次数额度               │
│     - 检查 Token 额度            │
│     - 超限 → 返回 402            │
└──────────────┬──────────────────┘
               │
               ▼
┌─────────────────────────────────┐
│  5. 模型路由                     │
│     - 根据请求中的 model 字段    │
│     - 查找映射 → source_name     │
│     - 查找可用渠道               │
│     - 未找到 → 返回 404          │
└──────────────┬──────────────────┘
               │
               ▼
┌─────────────────────────────────┐
│  6. 渠道选择（负载均衡）         │
│     - 按权重/轮询选择渠道        │
│     - 检查渠道限流               │
│     - 检查渠道配额               │
│       （次数/Token 超限 → 402） │
└──────────────┬──────────────────┘
               │
               ▼
┌─────────────────────────────────┐
│  7. 请求头伪装                     │
│     - 根据渠道绑定的 app_profile   │
│     - 替换 User-Agent              │
│     - 注入预设的 extra_headers     │
│     - 设置正确的 API Key           │
└──────────────┬──────────────────┘
               │
               ▼
┌─────────────────────────────────┐
│  8. 转发到上游供应商              │
│     - 支持 Streaming             │
│     - 超时控制                   │
│     - 重试机制                   │
└──────────────┬──────────────────┘
               │
               ▼
┌─────────────────────────────────┐
│  9. 响应处理                     │
│     - 解析响应，统计 Token       │
│     - 扣减配额                   │
│     - 记录日志                   │
│     - 返回给客户端               │
└─────────────────────────────────┘
```

### 6.2 限流实现（governor）

```rust
// 全局限流器
struct GlobalRateLimiter {
    qps_limiter: NonZeroGovernor,      // QPS 令牌桶
    concurrency: Arc<AtomicUsize>,      // 当前并发数
    max_concurrency: usize,             // 最大并发
}

// 用户限流器（Map 存储）
type UserRateLimiters = DashMap<String, Arc<UserRateLimiter>>;

// 渠道限流器
type ChannelRateLimiters = DashMap<i64, Arc<ChannelRateLimiter>>;

// 限流中间件逻辑
async fn rate_limit_middleware(req: Request, next: Next) -> Response {
    // 1. 检查全局
    if !GLOBAL.check() { return 429; }
    
    // 2. 检查用户
    let user_id = req.extensions().get::<UserId>().unwrap();
    if !USER_LIMITERS.get(user_id).check() { return 429; }
    
    // 3. 检查渠道（代理时检查）
    let channel_id = ctx.channel_id;
    if !CHANNEL_LIMITERS.get(channel_id).check() { return 429; }
    
    next.run(req).await
}
```

### 6.3 配额检查与扣减

```rust
// 检查配额
async fn check_quota(user_id: i64, estimated_tokens: u64) -> Result<()> {
    let quotas = db::get_active_quotas(user_id).await?;
    
    for quota in &quotas {
        if quota.quota_type == "requests" {
            if quota.used >= quota.total_limit {
                return Err(QuotaExceeded);
            }
        }
        if quota.quota_type == "tokens" {
            if quota.used + estimated_tokens > quota.total_limit {
                return Err(QuotaExceeded);
            }
        }
    }
    Ok(())
}

// 扣减配额（响应成功后）
async fn deduct_quota(user_id: i64, tokens: u64) {
    let quotas = db::get_active_quotas(user_id).await.unwrap();
    for quota in &quotas {
        if quota.quota_type == "requests" {
            db::increment_used(quota.id, 1).await;
        }
        if quota.quota_type == "tokens" {
            db::increment_used(quota.id, tokens).await;
        }
    }
}
```

---

## 七、前端页面详细设计

### 7.1 SoybeanAdmin 集成

```
# 使用 SoybeanAdmin 作为基础模板
# 1. 克隆 SoybeanAdmin 到 frontend/ 目录
# 2. 按以下结构修改

frontend/
├── src/
│   ├── api/              # 所有后端 API 调用
│   │   ├── auth.ts
│   │   ├── system.ts
│   │   ├── channel.ts
│   │   ├── model.ts
│   │   ├── user.ts
│   │   ├── quota.ts
│   │   ├── ratelimit.ts
│   │   ├── app_profile.ts     # 应用伪装预设
│   │   ├── stat.ts
│   │   └── log.ts
│   ├── views/            # 页面视图
│   │   ├── _builtin/     # Soybean 内置页面（登录、403、404、500）
│   │   ├── home/         # 仪表盘首页
│   │   ├── system/       # 分组页面
│   │   │   ├── settings/ # 系统设置
│   │   │   ├── ratelimit/# 速率限制
│   │   │   └── app_profiles/# 应用伪装预设
│   │   ├── channel/      # 渠道管理
│   │   ├── model/        # 模型管理
│   │   ├── user/         # 用户管理
│   │   ├── quota/        # 配额管理
│   │   ├── stat/         # 调用统计
│   │   └── log/          # 日志管理
│   ├── store/            # Pinia stores
│   └── router/           # 路由配置
```

### 7.2 路由设计

| 路由路径 | 页面 | 权限 | 说明 |
|----------|------|------|------|
| `/login` | 登录页 | 公开 | 用户名+密码登录 |
| `/home` | 仪表盘 | 管理员 | 统计数据概览 |
| `/system/settings` | 系统设置 | 管理员 | 服务器配置、全局请求头、日志级别 |
| `/system/ratelimit` | 限流配置 | 管理员 | 全局/用户/渠道限流规则 |
| `/system/app-profiles` | 应用伪装 | 管理员 | 管理应用伪装预设（UA + 请求头模板） |
| `/channel` | 渠道列表 | 管理员 | 渠道增删改查、测试连通性 |
| `/model` | 模型列表 | 管理员 | 模型映射、启用/禁用 |
| `/user` | 用户列表 | 管理员 | 用户增删改查、生成 Key、重置 Key |
| `/quota` | 配额管理 | 管理员 | 用户额度配置 |
| `/stat/usage` | 调用统计 | 管理员 | 按时间/用户/模型筛选、图表 |
| `/stat/export` | 数据导出 | 管理员 | 导出 CSV |
| `/log` | 调用日志 | 管理员 | 日志查看、筛选、详情 |

### 7.3 页面字段定义

#### 7.3.1 登录页

| 字段 | 类型 | 校验 |
|------|------|------|
| 用户名 | Input | 必填，4-32字符 |
| 密码 | Input.Password | 必填 |

**无注册按钮，只有登录按钮**。初始管理员通过环境变量或首次启动时命令行参数设置。

#### 7.3.2 仪表盘

| 模块 | 数据 | 展示方式 |
|------|------|----------|
| 总调用量 | 数字卡片 | 大数字 |
| 今日调用量 | 数字卡片 | 大数字 |
| 活跃用户数 | 数字卡片 | 大数字 |
| 错误率 | 数字卡片 | 百分比 + 颜色 |
| 近7日调用趋势 | 折线图 | ECharts |
| 渠道用量分布 | 饼图 | ECharts |
| 热门模型 Top5 | 列表 | 排序列表 |
| 实时请求 | 表格 | 最新10条 |

#### 7.3.3 渠道管理

**渠道列表表格**

| 列 | 说明 |
|------|------|
| 名称 | 渠道名 |
| 类型 | 标签（OpenAI/Anthropic） |
| Base URL | 地址 |
| API Keys | 数量 |
| 模型数 | 该渠道绑定的模型数 |
| 应用伪装 | 预设名称标签 |
| 权重 | 数字 |
| **渠道配额** | **进度条 + 文本（如 "150/1000 次/月"）** |
| 状态 | 标签（启用/禁用） |
| 操作 | 编辑、删除、测试、上下线 |

**渠道表单**

| 字段 | 类型 | 说明 |
|------|------|------|
| 名称 | Input | 渠道名称 |
| 类型 | Select | openai / anthropic |
| Base URL | Input | API 基础地址（**支持从预设供应商快速选择**） |
| API Keys | Textarea | 每行一个 Key |
| 应用伪装 | Select | 选择 app_profile（Claude Code / OpenCode / OpenClaw 等） |
| 自定义请求头 | 动态表单一组 | Key-Value 对，可添加删除（不使用预设时填写） |
| 权重 | InputNumber | 默认 1 |
| 超时 | InputNumber | ms，默认 300000 |
| 重试次数 | InputNumber | 默认 1 |
| 状态 | Switch | 启用/禁用 |
| **渠道配额类型** | **Select** | **requests（次数）/ tokens（Token）/ unlimited（不限）** |
| **渠道配额上限** | **InputNumber** | **与配额类型配合使用** |
| **渠道配额周期** | **Select** | **hourly / daily / weekly / monthly / permanent** |

**测试功能**：点击"测试"按钮，发送一个最小请求到该渠道，返回成功/失败及耗时。

#### 7.3.4 模型管理

**模型列表表格**

| 列 | 说明 |
|------|------|
| 中转模型名 | proxy_name |
| 原始模型名 | source_name |
| 所属渠道 | 渠道名称 |
| 状态 | 标签（启用/禁用） |
| 是否默认 | 标签 |
| 操作 | 编辑、删除、上下线 |

#### 7.3.5 用户管理

**用户列表表格**

| 列 | 说明 |
|------|------|
| 用户名 | username |
| API Key | 显示前8位，其余**** |
| 角色 | 标签 |
| 绑定模型 | 标签组 |
| 状态 | 标签（启用/禁用） |
| 配额使用 | 进度条 |
| 创建时间 | 日期 |
| 操作 | 编辑、删除、重置Key、管理配额 |

**用户表单**

| 字段 | 类型 | 说明 |
|------|------|------|
| 用户名 | Input | 必填，4-32字符 |
| 角色 | Select | admin / user |
| 初始密码 | Input.Password | 必填（创建时） |
| 状态 | Switch | 启用/禁用 |
| 备注 | Textarea | 可选 |
| 绑定模型 | TreeSelect/MultiSelect | 选择可用的模型 |

**API Key 生成**：新建用户时自动生成 `lp-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` 格式 Key，可显示和复制。

#### 7.3.6 配额管理

**配额列表**

| 列 | 说明 |
|------|------|
| 用户 | 用户名 |
| 类型 | 标签（次数/Token） |
| 总额度 | 数字 |
| 已使用 | 数字 + 进度条 |
| 使用率 | 百分比 |
| 周期 | 自定义/每月 |
| 时间区间 | 开始~结束 |
| 状态 | 启用/禁用 |
| 操作 | 编辑、删除、重置 |

**配额表单**

| 字段 | 类型 | 说明 |
|------|------|------|
| 用户 | Select | 必填 |
| 类型 | Select | requests（次数）/ tokens（Token），**二选一，不可同时设置** |
| 额度上限 | InputNumber | 必填 |
| 周期类型 | Select | **hourly（每小时）/ daily（每天）/ weekly（每周）/ monthly（每月）/ permanent（永久）** |
| 开始时间 | DatePicker | permanent 以外的周期类型时可选 |
| 结束时间 | DateTimePicker | permanent 以外的周期类型时可选 |

#### 7.3.7 应用伪装管理

**预设列表表格**

| 列 | 说明 |
|------|------|
| 名称 | 预设名称 |
| 标识符 | identifier |
| User-Agent | 显示前 60 字符 |
| 额外请求头 | 数量 |
| 来源 | 标签（系统内置 / 自定义） |
| 状态 | 启用/禁用 |
| 操作 | 编辑、删除（系统内置不可删）、上下线 |

**预设表单**

| 字段 | 类型 | 说明 |
|------|------|------|
| 名称 | Input | 预设显示名称 |
| 标识符 | Input | 唯一标识符（小写字母+连字符） |
| User-Agent | Input | 完整的 UA 字符串 |
| 额外请求头 | 动态表单一组 | Key-Value 对，如 `anthropic-beta`, `anthropic-version` 等 |
| 描述 | Textarea | 说明该预设的用途 |
| 启用 | Switch | 启用/禁用 |

**系统内置预设不可删除**，但可以禁用。自定义预设可完整管理（增删改）。

#### 7.3.8 限流配置

**限流规则列表**

| 列 | 说明 |
|------|------|
| 目标类型 | 标签（全局/用户/渠道） |
| 目标名称 | 用户名或渠道名 |
| QPS | 数字（0=不限） |
| 并发数 | 数字（0=不限） |
| 动作 | 标签（拒绝/排队） |
| 操作 | 编辑、删除 |

#### 7.3.8 调用统计

| 筛选条件 | 类型 |
|----------|------|
| 时间范围 | DatePicker.RangePicker |
| 用户 | Select |
| 模型 | Select |
| 渠道 | Select |

**统计卡片**

| 卡片 | 说明 |
|------|------|
| 总调用次数 | 数字 |
| 成功次数 | 数字 |
| 失败次数 | 数字 |
| 总 Token | 数字 |
| 平均耗时 | 数字（ms） |
| P95 耗时 | 数字（ms） |
| 错误率 | 百分比 |

**图表**

| 图表 | 类型 | 说明 |
|------|------|------|
| 调用趋势 | 折线图 | 按小时/天展示 |
| Token 消耗趋势 | 堆叠柱状图 | 输入/输出 Token |
| 用户调用排名 | 柱状图 | TOP 10 用户 |
| 模型调用分布 | 饼图 | 各模型比例 |

**导出按钮**：导出当前筛选条件下的数据为 CSV。

#### 7.3.9 日志管理

**日志列表表格**

| 列 | 说明 |
|------|------|
| 时间 | 创建时间 |
| 用户 | 用户名 |
| 模型 | 模型名 |
| 端点 | /v1/chat/completions 等 |
| 状态码 | 200/400/429/500 等，带颜色 |
| 耗时 | ms |
| 输入Token | 数字 |
| 输出Token | 数字 |
| 操作 | 查看详情 |

**筛选条件**

| 条件 | 类型 |
|------|------|
| 时间范围 | DatePicker.RangePicker |
| 用户 | Select |
| 模型 | Select |
| 状态码 | Select（2xx/4xx/5xx） |
| 端点 | Select |

**日志详情弹窗/抽屉**

| 字段 | 说明 |
|------|------|
| 基本信息 | 时间、用户、模型、端点 |
| 请求体 | JSON 格式化展示（摘要） |
| 响应体 | JSON 格式化展示（摘要） |
| Token 明细 | 输入/输出/总计 |
| 耗时 | 总耗时 |
| 错误信息 | 如有 |

---

## 八、配置管理

### 8.1 应用配置（config/config.yaml）

```yaml
# Server
server:
  port: 9000
  host: "0.0.0.0"

# Database
database:
  path: "./data/liteproxy.db"          # SQLite 文件路径
  pool_size: 10                          # 连接池大小

# Admin (初始管理员)
admin:
  username: "admin"
  password: "admin123"                   # 首次登录后建议修改

# JWT
jwt:
  secret: "change-me-to-random-string"   # JWT 密钥
  expires_in: 86400                      # 过期时间（秒）

# Log
log:
  level: "info"                          # trace/debug/info/warn/error
  retention_days: 30                     # 日志保留天数
  max_records: 100000                    # 最大日志记录数

# Proxy
proxy:
  timeout: 300000                        # 默认超时(ms)
  max_retries: 1                         # 最大重试次数
  log_request_body: false                # 是否记录请求体
  log_response_body: false               # 是否记录响应体

# Global Rate Limit
global_rate_limit:
  qps: 100                               # 0 = 不限
  concurrency: 50                        # 0 = 不限
  action: "reject"                       # reject / queue
```

### 8.2 环境变量覆盖

| 环境变量 | 说明 |
|----------|------|
| `LP_PORT` | 服务器端口 |
| `LP_DB_PATH` | 数据库路径 |
| `LP_ADMIN_USERNAME` | 管理员用户名 |
| `LP_ADMIN_PASSWORD` | 管理员密码 |
| `LP_JWT_SECRET` | JWT 密钥 |
| `LP_LOG_LEVEL` | 日志级别 |

---

## 八、预设供应商配置

### 8.1 内置供应商预设

系统内置以下供应商的 Base URL 配置，创建渠道时可一键选择。每个供应商提供两套独立的 Base URL：**普通版**和 **CodingPlan 版**，每套均包含 OpenAI 兼容和 Anthropic 兼容两个端点。

#### 普通版

| 渠道 | 类型 | Base URL | 说明 |
|------|------|----------|------|
| 阿里云百炼 | openai | `https://dashscope.aliyuncs.com/compatible-mode/v1` | 通义千问系列，OpenAI 兼容 |
| 阿里云百炼 | anthropic | `https://dashscope.aliyuncs.com/compatible-mode/v1/apps/anthropic` | 通义千问系列，Anthropic 兼容 |
| 智谱清言 | openai | `https://open.bigmodel.cn/api/paas/v4` | GLM 系列，OpenAI 兼容 |
| 智谱清言 | anthropic | `https://open.bigmodel.cn/api/paas/v4/anthropic` | GLM 系列，Anthropic 兼容 |
| Kimi（月之暗面） | openai | `https://api.moonshot.cn/v1` | Kimi 系列，OpenAI 兼容 |
| Kimi（月之暗面） | anthropic | `https://api.moonshot.cn/v1/anthropic` | Kimi 系列，Anthropic 兼容 |
| 小米 | openai | `https://api.miapi.xiaomi.com/v1` | 小米大模型，OpenAI 兼容 |
| 小米 | anthropic | `https://api.miapi.xiaomi.com/v1/anthropic` | 小米大模型，Anthropic 兼容 |
| MiniMax | openai | `https://api.minimaxi.com/v1` | MiniMax 系列，OpenAI 兼容 |
| MiniMax | anthropic | `https://api.minimaxi.com/v1/anthropic` | MiniMax 系列，Anthropic 兼容 |

#### CodingPlan 版

| 渠道 | 类型 | Base URL | 说明 |
|------|------|----------|------|
| 阿里云 CodingPlan | openai | `https://coding.dashscope.aliyuncs.com/v1` | 阿里云 codingplan，按调用次数计费 |
| 阿里云 CodingPlan | anthropic | `https://coding.dashscope.aliyuncs.com/apps/anthropic` | 阿里云 codingplan，Anthropic 兼容 |
| 智谱 CodingPlan | openai | `https://open.bigmodel.cn/api/paas/v4` | 智谱 codingplan（使用普通版 Base URL） |
| 智谱 CodingPlan | anthropic | `https://open.bigmodel.cn/api/anthropic` | 智谱 codingplan，Anthropic 兼容（独立域名路径） |
| Kimi CodingPlan | openai | `https://api.kimi.com/coding/` | Kimi codingplan，独立域名，按调用次数计费 |
| Kimi CodingPlan | anthropic | `https://api.kimi.com/coding/` | Kimi codingplan，Anthropic 兼容（同 Base URL） |
| 小米 CodingPlan | openai | `https://token-plan-cn.xiaomimimo.com/v1` | 小米 codingplan，独立域名 |
| 小米 CodingPlan | anthropic | `https://token-plan-cn.xiaomimimo.com/anthropic` | 小米 codingplan，Anthropic 兼容 |
| MiniMax CodingPlan | openai | `https://api.minimaxi.com/v1/coding` | MiniMax codingplan（待确认准确地址） |
| MiniMax CodingPlan | anthropic | `https://api.minimaxi.com/v1/coding/anthropic` | MiniMax codingplan（待确认准确地址） |

> 以上预设仅提供 Base URL 和类型，API Key 由管理员自行填写。
> CodingPlan 版本与普通版本的区别在于配额计算方式不同：CodingPlan 渠道默认按 `requests`（次数）配额，且有明确的周期限制（时/日/周/月）。

### 8.2 使用方式

1. 新建渠道时，选择「从预设导入」
2. 先选择供应商（阿里云/智谱/Kimi/小米/MiniMax）
3. 再选择版本类型：**普通版** 或 **CodingPlan 版**
4. 再选择接口类型：**OpenAI 兼容** 或 **Anthropic 兼容**，自动填充对应的 Base URL 和类型
5. 填写 API Key
6. 设置渠道配额（CodingPlan 版默认推荐按 `requests` 次数配额，配合时/日/周/月周期）
7. 选择应用伪装预设（如 Claude Code）
8. 保存即可

> 以上预设仅提供 Base URL 和类型，API Key 由管理员自行填写。
> CodingPlan 版本与普通版本的区别在于：Base URL 不同、配额计算方式不同。CodingPlan 渠道推荐按 `requests`（次数）配额，且有明确的周期限制（时/日/周/月）。

---

## 九、部署

### 9.1 Docker 构建

**Dockerfile.backend**
```dockerfile
FROM rust:1.84-slim AS builder
WORKDIR /app
COPY backend/Cargo.* ./
COPY backend/src ./src
RUN cargo build --release
RUN cp target/release/liteproxy /liteproxy

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /liteproxy /usr/local/bin/liteproxy
WORKDIR /data
EXPOSE 9000
CMD ["liteproxy"]
```

**Dockerfile.frontend**
```dockerfile
FROM node:22-slim AS builder
WORKDIR /app
COPY frontend/package*.json ./
RUN npm install
COPY frontend/ .
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

**docker-compose.yml**
```yaml
version: "3.8"
services:
  backend:
    build:
      dockerfile: deploy/Dockerfile.backend
    ports:
      - "9000:9000"
    volumes:
      - liteproxy-data:/data
      - ./config:/app/config
    environment:
      - LP_JWT_SECRET=${LP_JWT_SECRET:-change-me}
    restart: unless-stopped

  frontend:
    build:
      dockerfile: deploy/Dockerfile.frontend
    ports:
      - "80:80"
    depends_on:
      - backend
    restart: unless-stopped

volumes:
  liteproxy-data:
```

---

## 十、开发指南

### 10.1 开发启动

```bash
# 后端开发
cd backend
cargo run

# 前端开发
cd frontend
npm install
npm run dev

# 数据库迁移
cd backend
cargo sqlx migrate run
```

### 10.2 SQLx 离线模式

```bash
# 准备 SQLx 离线数据（用于编译期检查）
cd backend
cargo sqlx prepare

# 迁移
cargo sqlx migrate add 001_init
cargo sqlx migrate run
```

### 10.3 前端 SoybeanAdmin 初始化

```bash
# 克隆 SoybeanAdmin
git clone https://github.com/soybeanjs/soybean-admin.git frontend

# 进入目录
cd frontend

# 安装依赖
pnpm install

# 启动开发服务器
pnpm dev

# 根据本方案修改 API 调用路径、路由、视图等
```

---

## 十一、测试方案

### 11.1 后端测试

| 类型 | 工具 | 说明 |
|------|------|------|
| 单元测试 | `cargo test` | 模块级别测试 |
| 集成测试 | `cargo test --test` | API 端点测试 |
| Mock 供应商 | `mockito` | Mock OpenAI/Anthropic 响应 |

### 11.2 测试清单

- [ ] API Key 鉴权测试（正确 Key、错误 Key、禁用用户）
- [ ] 限流测试（触发 429）
- [ ] 配额测试（超限 402）
- [ ] OpenAI 接口透传测试
- [ ] Anthropic 接口透传测试
- [ ] Streaming 响应测试
- [ ] 请求头伪装测试
- [ ] 模型映射测试
- [ ] 多 Key 轮询测试
- [ ] 错误重试测试
- [ ] 超时测试

---

## 十二、错误码规范

| 状态码 | 含义 |
|--------|------|
| 200 | 成功 |
| 400 | 请求参数错误 |
| 401 | 未认证（无效 Key） |
| 402 | 配额已超 |
| 403 | 权限不足（用户禁用/模型不可用） |
| 404 | 模型/渠道未找到 |
| 429 | 限流触发 |
| 500 | 服务器内部错误 |
| 502 | 上游供应商错误 |
| 504 | 上游超时 |

---

## 十三、Agent 开发指引

> 以下供 AI Agent 开发工具参考。

### 13.1 开发顺序

1. **后端基础** → Cargo.toml、config、db 初始化、SQLx 迁移
2. **数据模型** → SQLx models、CRUD 操作
3. **中间件** → Auth、RateLimit、RequestHeader
4. **代理核心** → proxy service、OpenAI/Anthropic 适配
5. **管理 API** → admin routes（CRUD）
6. **前端基础** → SoybeanAdmin 初始化、API 封装、路由配置
7. **前端页面** → 按模块逐个开发
8. **测试** → 单元测试 + 集成测试
9. **部署** → Dockerfile、docker-compose

### 13.2 编码规范

| 规范 | 要求 |
|------|------|
| Rust | `cargo fmt` + `cargo clippy` 无警告 |
| 错误处理 | 使用 `thiserror` 定义业务错误，统一 error response 格式 |
| API 响应 | 统一 JSON 格式：`{"code": 0, "message": "ok", "data": {...}}` |
| 前端 | Vue3 Composition API + `<script setup lang="ts">` |
| TypeScript | strict mode，no `any` |
| 注释 | 公共函数/结构体必须有文档注释 |

### 13.3 Git 提交规范

```
feat: 新增功能
fix: 修复 bug
refactor: 重构代码
docs: 文档更新
test: 测试相关
chore: 构建/工具链相关
```

---

## 十四、后续迭代计划（v2.x）

| 功能 | 优先级 |
|------|--------|
| Webhook 告警（额度预警、渠道宕机） | P1 |
| Prometheus Metrics 导出 | P1 |
| Redis 集群支持 | P2 |
| API 缓存（相同请求复用响应） | P2 |
| 多节点部署 | P2 |
| 插件系统（自定义供应商适配器） | P3 |
