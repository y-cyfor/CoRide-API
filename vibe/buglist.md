# CoRide-API Bug 审计报告 - 待修复缺陷

> 审计时间: 2026-04-17
> 已修复缺陷已迁移至 `buglist-completed.md`
> 此文件仅保留待处理问题

---

## 一、流程阻断性 Bug (P0 - 必须立即修复)

> 以下问题会导致用户操作流程无法进行

---

### 1. 用户路由使用了 admin 认证中间件 ❌

**问题描述：**
- `main.rs:88` 中 `user_routes` 使用了 `admin_auth_middleware`
- `admin_auth_middleware` (admin_auth.rs:64) 会检查 `role == "admin"`
- 普通用户访问 `/user/info` 和 `/user/keys` 会返回 403 Forbidden
- **后果**: 普通用户无法管理自己的 API Key

**涉及文件：** `backend/src/main.rs:82-88`

**修复方案：** 创建一个只验证 JWT 但不要求 admin 角色的中间件 `user_auth_middleware`，用于 user_routes

---

### 2. Key 管理路由权限配置为 admin only ❌

**问题描述：**
- `routes.ts:226-247` 中 `/user` 路由设置了 `roles: ['admin']`
- `user_key` 子路由也设置了 `roles: ['admin']`
- 普通用户登录后在菜单中看不到 Key 管理页面
- **后果**: 普通用户无法访问自己的 Key 管理功能

**涉及文件：** `web/src/router/elegant/routes.ts:226-247`

**修复方案：** 
- 将 `/user` 路由的 `roles` 改为空数组或移除（表示全员可见）
- 将 `/user/key` 路由的 `roles` 改为空数组

---

### 3. 仪表盘页面调用 admin API 导致普通用户 403 ❌

**问题描述：**
- `home/index.vue:49-100` 调用了多个 admin 接口：
  - `fetchChannelList()` → `/admin/channels` (line 49)
  - `fetchModelList()` → `/admin/models` (line 57)
  - `fetchDashboardStats()` → `/admin/stats/dashboard` (line 71)
  - `fetchUsageStats()` → `/admin/stats/usage` (line 72)
  - `fetchRecentLogs()` → `/admin/logs` (line 92)
  - `fetchQuotaWarnings()` → `/admin/quotas/warnings` (line 97)
- 普通用户访问这些接口会返回 403 Forbidden
- **后果**: 普通用户打开仪表盘页面会报错，页面无法正常显示

**涉及文件：** `web/src/views/home/index.vue`

**修复方案：**
- 为普通用户创建专用的统计数据 API（如 `/user/stats/dashboard`）
- 或在前端根据用户角色判断是否调用 admin API
- 移除普通用户不需要的筛选选项（渠道/模型筛选需要 admin 权限）

---

### 4. 数据统计页面调用 admin API ❌

**问题描述：**
- `data/stats/index.vue` 调用 `fetchUsageStats()` → `/admin/stats/usage`
- `data/log/index.vue` 调用 `fetchLogList()` → `/admin/logs`
- 普通用户访问会返回 403 Forbidden
- **后果**: 普通用户无法查看统计数据和日志

**涉及文件：** 
- `web/src/views/data/stats/index.vue`
- `web/src/views/data/log/index.vue`

**修复方案：**
- 后端创建 `/user/stats` 和 `/user/logs` 接口，只返回当前用户的数据
- 前端根据用户角色调用不同的 API

---

### 5. 后端缺少普通用户专用 API ❌

**问题描述：**
- 后端所有统计和日志 API 都在 `admin_protected` 路由组下
- 普通用户没有可调用的统计数据接口
- `proxy_routes.rs:336` 有 `user_info` 但不是统计接口
- **后果**: 前端仪表盘/统计页面对普通用户完全不可用

**涉及文件：** `backend/src/main.rs`

**修复方案：** 创建普通用户可访问的 API：
- `/user/stats/dashboard` - 返回当前用户的统计数据
- `/user/stats/usage` - 返回当前用户的使用趋势
- `/user/logs` - 返回当前用户的请求日志

---

## 二、待手动处理 (安全相关)

### 1. 硬编码敏感信息 ~~[需手动处理]~~

**文件**: `backend/config/config.yaml:12-16`

**状态**: 配置已支持环境变量覆盖，需在部署时配置

---

## 三、设计如此 (非Bug)

### 1. set_log_level API 不实际修改日志级别 ~~[设计如此]~~

**原因**: Rust tracing-subscriber 不支持热重载，需要重启

### 2. set_global_rate_limit 修改配置文件但运行时不生效 ~~[设计如此]~~

**原因**: Governor RateLimiter 不支持动态修改，需要重启

### 3. 配额 total_limit 默认值为0 ~~[设计如此]~~

**原因**: 用户会手动修改默认值，属于 UI 设计建议

---

## 四、暂不修改 (技术限制/优先级低)

### 1. Streaming 响应全量加载到内存 ~~[暂不修改]~~

**原因**: 需要实现真正的流式转发，改动较大

### 2. Token 估算不考虑 JSON 结构 ~~[暂不修改]~~

**原因**: 当前按字符计数是合理的启发式估算

---

## 五、已修复缺陷

> 以下缺陷已修复并迁移至 `buglist-completed.md`

| 原问题 | 修复方案 | 迁移至 |
|--------|----------|--------|
| quota_warnings SQL 除零风险 | 添加 `total_limit > 0` WHERE 条件 | buglist-completed.md 八.1 |
| CSV 导出 channel_id=0 误导 | NULL 显示为 "N/A" | buglist-completed.md 八.2 |
| 流量计划随机数种子不够随机 | 混合多熵源 | buglist-completed.md 八.3 |
| 流量计划时段重叠未验证 | 后端添加 validate 函数 | buglist-completed.md 八.4 |
| Health Check 将 4xx 视为健康 | 仅 `is_success()` 视为健康 | buglist-completed.md 九.1 |
| update_user_key 代码冗余 | 简化 if/else 结构 | buglist-completed.md 九.2 |
| settings 日志级别误导用户 | 添加重启警告提示 | buglist-completed.md 九.3 |
| home total_tokens 类型不匹配 | stats ref 添加字段声明 | buglist-completed.md 九.4 |

---

## 六、已排除的"伪Bug"

| 原"问题" | 原因 |
|----------|------|
| test_channel 创建新 HTTP 客户端 | 低频管理功能，非 bug |
| update_quota 硬编码 enabled=true | 设计决策 |
| API Key 前缀不一致 | 设计决策 |
| quota loadUsers 未处理分页格式 | 后端返回数组，前端正确 |
| model loadData 未处理分页格式 | 后端返回数组，前端正确 |
| 统计页面 total_tokens 取值不一致 | 设计问题，非 bug |
| 渠道表单缺少配额默认值 | 编辑时从后端加载 |
| usage_stats 动态 SQL 过于复杂 | 可维护性建议 |
| export_logs_csv 固定10000条 | 功能限制 |
| monthly 配额周期=30天 | 设计决策 |
| set_log_level 不实际生效 | 设计如此 |
| set_global_rate_limit 不生效 | 设计如此 |
| 配额 total_limit 默认值=0 | 设计如此 |

---

## 七、待处理问题优先级

| 优先级 | 问题 | 风险 | 状态 |
|--------|------|------|------|
| **P0** | 用户路由使用 admin 中间件 | 普通用户无法管理 Key | ❌ 待修复 |
| **P0** | Key 管理路由权限配置为 admin | 普通用户看不到菜单 | ❌ 待修复 |
| **P0** | 仪表盘页面调用 admin API | 普通用户页面报错 | ❌ 待修复 |
| **P0** | 数据统计页面调用 admin API | 普通用户无法访问 | ❌ 待修复 |
| **P0** | 后端缺少普通用户专用 API | 前端无数据可调用 | ❌ 待修复 |
| P0 | 硬编码凭据 | 凭据泄露 | ⚠️ 需手动配置 |
| P1 | set_log_level 不实际生效 | 设计如此 | 📝 需重构 tracing |
| P1 | set_rate_limit 不生效 | 设计如此 | 📝 需重构 AppState |
| P2 | 配额默认值=0 | 设计如此 | 📝 UI 建议 |
| P3 | Streaming全量加载 | 暂不修改 | ⏸️ 技术限制 |
| P3 | Token估算不准确 | 暂不修改 | ⏸️ 技术限制 |

---

## 八、缺陷统计

| 类别 | 待修复 | 设计如此 | 暂不修改 | 需手动处理 | 合计 |
|------|--------|----------|----------|------------|------|
| 流程阻断 | 5 | 0 | 0 | 0 | 5 |
| 安全问题 | 0 | 0 | 0 | 1 | 1 |
| 设计缺陷 | 0 | 2 | 0 | 0 | 2 |
| 性能 | 0 | 0 | 1 | 0 | 1 |
| 其他 | 0 | 0 | 1 | 0 | 1 |
| **合计** | **5** | **2** | **2** | **1** | **10** |

---

> 已修复缺陷请查看 `buglist-completed.md`（共38项已修复）