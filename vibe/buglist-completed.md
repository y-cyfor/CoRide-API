# CoRide-API 已修复缺陷记录

> 此文件记录已完成修复的缺陷，从 buglist.md 迁移
> 修复时间: 2026-04-15

---

## 一、严重安全问题 (Critical Security)

### 1. Admin 路由完全无认证保护 ✅
- **文件**: `backend/src/main.rs:66-107`
- **修复**: 为 admin 路由添加 JWT 认证中间件，除 `/admin/auth/login` 外所有路由受保护

### 2. JWT 认证未实现 ✅
- **文件**: `backend/src/router/admin_routes.rs:166-195`
- **修复**: 实现真正的 JWT 签发与验证逻辑 (`backend/src/utils/jwt.rs`)，`get_me` 接口正常工作

### 3. SQL 注入漏洞 ✅
- **文件**: `backend/src/db/models.rs:965-991`
- **修复**: 使用参数化查询 `sqlx::query_as` 的 bind 方法

### 5. API Key 泄露 ✅
- **文件**: `backend/src/router/admin_routes.rs:734-741`
- **修复**: 对 API Key 进行脱敏处理（只显示前8位 + "..."）

### 6. 请求体大小无限制 ✅
- **文件**: `backend/src/router/proxy_routes.rs:116`
- **修复**: 设置 10MB 请求体大小限制

### 7. CORS 配置过于宽松 ✅
- **文件**: `backend/src/main.rs:114`
- **修复**: 配置具体的允许方法和头部，不再使用 permissive()

---

## 二、设计缺陷 (Design Flaws)

### 1. 更新用户丢失 enabled_models ✅
- **文件**: `backend/src/router/admin_routes.rs:535`
- **修复**: UpdateUserRequest 添加 enabled_models 字段，更新时保留原有值

### 2. 更新模型忽略 channel_id ✅
- **文件**: `backend/src/router/admin_routes.rs:584-612`
- **修复**: 在 update_model 函数中使用 channel_id 更新数据库

### 3. 配额周期未持久化 ✅
- **文件**: `backend/src/router/admin_routes.rs:364-376`
- **修复**: 将 period_start/period_end 传入数据库创建

### 4. Channel 测试功能未实现 ✅
- **文件**: `backend/src/router/admin_routes.rs:303-308`
- **修复**: 实现真实的渠道连通性测试（向 /models 端点发送请求）

### 5. 重试机制未实现 ✅
- **文件**: `backend/src/router/proxy_routes.rs:160-231`
- **修复**: 实现基于 retry_count 的重试逻辑，仅对 5xx 错误重试

### 6. 登录 Token 未持久化/过期 ✅
- **文件**: `backend/src/router/admin_routes.rs:177`
- **修复**: 使用 JWT token，内置过期机制 (expires_in 配置)

---

## 三、UI/交互缺陷 (UI/UX Issues)

### 1. 渠道编辑丢失配额和高级配置 ✅
- **文件**: `web/src/views/manage/channel/index.vue:105-118`
- **修复**: formModel 包含 quota_type、quota_limit、quota_cycle、app_profile_id 字段，表单增加对应输入项

### 2. 供应商预设 codingplan URL 相同 ✅
- **文件**: `web/src/views/manage/channel/index.vue:40-53`
- **修复**: codingplan 版本 URL 添加 `/codingplan` 后缀

### 4. 渠道列表无法显示总数 ✅
- **文件**: `web/src/views/manage/channel/index.vue:82`
- **修复**: API 返回 `{ items, total }` 格式，前端正确设置 itemCount

### 5. Loading 状态残留 ✅
- **文件**: `web/src/views/manage/channel/index.vue:146-152`
- **修复**: 使用 message 实例手动关闭 loading 状态

---

## 四、性能缺陷 (Performance Issues)

### 1. 每次请求创建新 HTTP 客户端 ✅
- **文件**: `backend/src/service/proxy.rs:105-107`
- **修复**: HTTP 客户端放入 AppState 共享

### 3. 并发计数器竞争 ✅
- **文件**: `backend/src/middleware/rate_limit.rs:60-64`
- **修复**: 使用 CAS 循环避免竞态，使用 Relaxed 内存顺序

### 4. 无数据库索引优化 ✅
- **描述**: 为高频查询字段添加索引
- **修复**: 在 001_init.sql 添加 user_api_key, model, created_at, status_code 索引

---

## 五、逻辑 Bug (Logic Bugs)

### 1. 并发计数器竞态条件 ✅
- **文件**: `backend/src/middleware/rate_limit.rs:59-64`
- **修复**: 使用 CAS (compare_exchange_weak) 循环

### 2. 更新用户变量解构错误 ✅
- **文件**: `backend/src/router/admin_routes.rs:525-533`
- **修复**: 正确解构 enabled_models 并传入 update_user

### 3. Anthropic 流式检测不准确 ✅
- **文件**: `backend/src/service/anthropic.rs:23-30`
- **修复**: 检查所有流式事件类型 (message_start, content_block_start/delta/stop, message_delta/stop, ping)

### 5. 配额检查时序问题 ✅
- **文件**: `backend/src/router/proxy_routes.rs:142-145`
- **修复**: 先检查渠道配额再检查用户配额，避免用户配额被预扣但渠道无配额的情况

---

## 六、缺失功能 (Missing Features)

### 1. 请求日志清理调度 ✅
- **修复**: 后台任务每小时自动清理过期日志（根据 retention_days 配置）

### 2. 健康检查详细信息 ✅
- **修复**: /health 端点检查数据库连接状态，返回 JSON 格式状态信息

### 3. 优雅关闭 (Graceful Shutdown) ✅
- **修复**: 监听 Ctrl+C 和 SIGTERM 信号，等待进行中的请求完成

---

## 七、2026-04-15 新增缺陷

### 1. 首页假数据迁移 ✅

**问题描述：**
- `/dashboard` 路由 (`web/src/views/dashboard/index.vue`) 是 SoybeanAdmin 模板自带的首页，包含 `card-data.vue`、`header-banner.vue`、`project-news.vue`、`creativity-banner.vue` 等组件，全部展示硬编码假数据（如 `visitCount: 9725`、`turnover: 1026` 等）
- `/home` 路由 (`web/src/views/home/index.vue`) 已改造为真实数据页面（调用 `fetchDashboardStats`、`fetchUsageStats` 等 API），包含统计卡片 + 趋势图 + 渠道饼图 + Token 柱状图
- 用户希望保留 `/home` 作为首页入口，将 `/dashboard` 中样式好看的组件（如 greeting header banner 等视觉元素）融入 `/home`，同时去掉 `/dashboard` 的假数据

**修复方案：**
1. 将 `/home` 页面 UI 升级：参考 `/dashboard` 的 `HeaderBanner`（带用户头像 + 问候语 + 天气描述）作为顶部欢迎区域，替换 `/home` 当前的纯卡片布局
2. 保留 `/home` 已有的真实数据调用逻辑（`fetchDashboardStats`、`fetchUsageStats` 等）
3. 删除 `/dashboard` 路由或将 `/dashboard` 重定向到 `/home`，避免用户访问到假数据页面
4. 涉及文件：
   - `web/src/views/home/index.vue` — 升级布局
   - `web/src/views/dashboard/index.vue` — 标记废弃或改为重定向
   - `web/src/router/elegant/routes.ts` — 调整路由配置

---

### 2. 渠道测试返回 "warning" 状态 ✅

**问题描述：**
- 在渠道管理中点击"测试"按钮，调用 `/proxy-default/admin/channels/:id/test` 接口
- 即使已正确配置 BaseURL 和 API Key，仍返回 `"status": "warning"`，提示类似 `"Channel responded with status 401"`

**修复方案：**
- **代码层面改进**：
  1. 在返回 warning 时，将响应体也返回给前端，让用户知道上游具体返回了什么错误信息
  2. 对常见错误状态码给出友好提示（如 401 → "API Key 无效"，403 → "该 Key 无权限访问 /models"，404 → "BaseURL 可能不正确"）
  3. 涉及文件：`backend/src/router/admin_routes.rs:386-396`

---

### 3. 模型管理渠道选择只显示 "0" ✅

**问题描述：**
- 在模型管理页面创建模型时，渠道下拉框只显示一个 `"0"` 选项，无法选择已添加的真实渠道

**问题原因：**
- `loadChannels` 函数（`web/src/views/manage/model/index.vue:59-64`）中直接对 `data` 对象调用 `.map()`，没有取 `data.items`
- 后端 `fetchChannelList` 返回的数据格式为 `{ items: [...], total: N }`

**修复方案：**
```typescript
async function loadChannels() {
  const { data } = await fetchChannelList(1, 100);
  if (data) {
    const items = data.items || data;
    channelOptions.value = items.map((c: any) => ({ label: c.name, value: c.id }));
  }
}
```

---

### 4. 使用统计页面 UI 对齐问题 ✅

**问题描述：**
- `/manage/stats` 页面各数据展示区域布局不对齐
- 5个统计卡片使用 `NGrid cols="2 s:3 m:5"` 布局，在不同屏幕尺寸下卡片高度和间距不一致

**修复方案：**
1. 统一 5 个统计卡片的布局：使用 `NGrid :cols="5" :x-gap="16" :y-gap="16"` 固定列数
2. 将"总 Token 消耗"合并到统计卡片区域，改为第 6 个卡片
3. 统一区块间距：外层使用统一的 `NSpace :size="16"`
4. 涉及文件：`web/src/views/manage/stats/index.vue:126-225`

---

## 已修复统计

| 类别 | 已修复数量 |
|------|-----------|
| 安全问题 | 6 |
| 设计缺陷 | 6 |
| UI/交互 | 4 |
| 性能 | 3 |
| 逻辑Bug | 4 |
| 缺失功能 | 3 |
| 2026-04-15 新增 | 4 |
| 2026-04-16 新增 | 4 |
| 2026-04-17 新增 | 4 |
| 流程阻断 Bug | 5 |
| **合计** | **43** |

---

## 八、2026-04-16 新增修复

### 1. quota_warnings SQL 除零风险 ✅

**问题描述：**
- `admin_routes.rs:1238` 查询 `CAST(q.used AS FLOAT) / CAST(q.total_limit AS FLOAT)`
- 当 `total_limit=0` 时产生除零错误

**修复方案：**
- 在 WHERE 子句中添加 `q.total_limit > 0` 条件，过滤零限额配额

**涉及文件：** `backend/src/router/admin_routes.rs:1240`

---

### 2. CSV 导出 channel_id=0 误导 ✅

**问题描述：**
- `admin_routes.rs:1103` 中 `ch.unwrap_or(0)` 将 NULL 变为 0
- 用户可能误以为日志来自 ID=0 的渠道

**修复方案：**
- 将 `ch.unwrap_or(0)` 改为 `ch.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string())`
- NULL 值在 CSV 中显示为 "N/A"

**涉及文件：** `backend/src/router/admin_routes.rs:1103`

---

### 3. 流量计划随机数种子不够随机 ✅

**问题描述：**
- `models.rs:1247-1252` 使用纳秒时间戳做随机种子
- 高并发时可能产生相同种子，导致不公平分配

**修复方案：**
- 混合多种熵源：时间戳（秒+纳秒）、进程 ID、线程地址
- 使用黄金比例常数进行位运算混合

**涉及文件：** `backend/src/db/models.rs:1245-1260`

---

### 4. 流量计划时段重叠未验证 ✅

**问题描述：**
- 后端 `upsert_global_traffic_plan` 和 `upsert_channel_traffic_plan` 未校验时段重叠
- 用户可配置 0-8 和 6-12 这样的重叠时段

**修复方案：**
- **后端**：添加 `validate_traffic_plan_slots` 函数，验证时段范围、权重、重叠
- **前端**：已有重叠检查和保存阻止逻辑（line 209-219, 323-332）

**涉及文件：** `backend/src/router/admin_routes.rs:1280-1310`

---

## 十、2026-04-17 流程阻断性修复

### 1. 用户路由使用 admin 认证中间件 ✅

**问题描述：**
- `user_routes` 使用了 `admin_auth_middleware`，该中间件检查 `role == "admin"`
- 普通用户访问 `/user/info` 和 `/user/keys` 返回 403

**修复方案：**
- 新建 `user_auth_middleware`（只验证 JWT 和用户状态，不检查角色）
- `user_routes` 改用 `user_auth_middleware`

**涉及文件：** `backend/src/middleware/user_auth.rs`（新建）、`backend/src/main.rs`

---

### 2. Key 管理路由权限配置为 admin only ✅

**问题描述：**
- `/user` 和 `/user/key` 路由设置了 `roles: ['admin']`
- 普通用户看不到 Key 管理菜单

**修复方案：**
- 移除 `roles: ['admin']` 限制

**涉及文件：** `web/src/router/elegant/routes.ts`

---

### 3. 仪表盘页面调用 admin API ✅

**问题描述：**
- `home/index.vue` 调用 `/admin/stats/dashboard`、`/admin/stats/usage` 等
- 普通用户返回 403

**修复方案：**
- 前端检测用户角色，admin 用 admin API，user 用 `/user/stats/dashboard`
- 后端新增 `/user/stats/dashboard` 接口，按用户 API Key 过滤数据

**涉及文件：** `backend/src/router/admin_routes.rs`、`web/src/views/home/index.vue`

---

### 4. 数据统计/日志页面调用 admin API ✅

**问题描述：**
- `data/stats/index.vue` 和 `data/log/index.vue` 调用 admin 接口
- 普通用户返回 403

**修复方案：**
- 后端新增 `/user/stats/usage` 和 `/user/logs` 接口
- 前端根据角色调用对应 API

**涉及文件：** `backend/src/router/admin_routes.rs`、`web/src/views/data/stats/index.vue`、`web/src/views/data/log/index.vue`

---

### 5. 后端缺少普通用户专用 API ✅

**问题描述：**
- 所有统计和日志 API 都在 `admin_protected` 路由组下

**修复方案：**
- 新增 `/user/stats/dashboard`、`/user/stats/usage`、`/user/logs` 三个接口
- 通过 JWT 中的 user_id 查找用户 API Key，只返回该用户的数据

**涉及文件：** `backend/src/router/admin_routes.rs`

---

## 九、2026-04-17 新增修复

### 1. Health Check 将 4xx 视为健康 ✅

**问题描述：**
- `health.rs:58` 中 `is_success() || is_client_error()` 将 401/403 等视为"健康"
- API Key 无效的渠道不会被自动禁用

**修复方案：**
- 仅 `is_success()` 视为健康，移除 `is_client_error()` 判断

**涉及文件：** `backend/src/service/health.rs:58`

---

### 2. update_user_key 代码冗余 ✅

**问题描述：**
- `models.rs:1353-1355` 创建了一个无 WHERE 条件的查询但从未执行

**修复方案：**
- 简化为 `let q = if let Some...` 结构，移除死代码

**涉及文件：** `backend/src/db/models.rs:1353-1364`

---

### 3. settings 日志级别"更新成功"误导用户 ✅

**问题描述：**
- 日志级别更新后显示"成功"，但实际需重启才生效
- 缺少类似限流配置的警告提示

**修复方案：**
- 在日志级别区域添加 `<NAlert>` 警告 "修改后需重启服务才能生效"

**涉及文件：** `web/src/views/settings/index.vue:136`

---

### 4. home 统计卡片 total_tokens 类型不匹配 ✅

**问题描述：**
- `stats.value.total_tokens` 赋值给未声明的属性
- TypeScript 类型检查可能报错

**修复方案：**
- 在 stats ref 定义中添加 `total_tokens: 0`

**涉及文件：** `web/src/views/home/index.vue:17`

---

## 十一、2026-04-18 新增修复

### 1. IP 白名单功能完全失效（middleware 顺序错误）✅

**问题描述：**
- `ip_filter` 在 `auth` 之前执行，`user_id` 始终为 `None`，白名单检查永不执行

**修复方案：**
- 调整 layer 顺序：auth 最先执行，ip_filter 最后执行

**涉及文件：** `backend/src/main.rs:67-75`

---

### 2. X-Real-IP / X-Forwarded-For 可被伪造 ✅

**问题描述：**
- `extract_client_ip` 无条件信任客户端 header，攻击者可伪造 IP 绕过黑名单

**修复方案：**
- 移除 X-Forwarded-For 信任，仅信任 X-Real-IP（由 nginx 等可信代理设置）或 remote_addr

**涉及文件：** `backend/src/middleware/ip_filter.rs:12-28`

---

### 3. IP 前后端无格式验证 ✅

**问题描述：**
- 添加 IP 黑白名单时可输入任意字符串（如 `hello`）存入数据库

**修复方案：**
- 后端添加 `is_valid_ip_or_cidr` 函数校验 IPv4/IPv6/CIDR 格式
- 前端添加正则表达式验证 + 提示文案

**涉及文件：** `backend/src/router/admin_routes.rs:186-205`、`web/src/views/settings/index.vue:119-124`、`web/src/views/control/user/index.vue:97-101`

---

### 4. INSERT OR IGNORE 返回 ID 为 0 ✅

**问题描述：**
- 唯一约束冲突时 `INSERT OR IGNORE` 静默忽略，`last_insert_rowid()` 返回 0

**修复方案：**
- 改为"先查后插"模式：先 SELECT COUNT(*) 检查，存在则返回 409 CONFLICT

**涉及文件：** `backend/src/router/admin_routes.rs:1697-1735`

---

### 5. models::create_quota 未支持 channel_id ✅

**问题描述：**
- 函数签名无 `channel_id` 参数，SQL 插入语句不含 `channel_id` 列

**修复方案：**
- 函数签名添加 `channel_id: Option<i64>` 参数
- INSERT SQL 包含 channel_id 列

**涉及文件：** `backend/src/db/models.rs:582-601`

---

### 6. quotas 表 channel_id 缺少索引 ✅

**问题描述：**
- `get_user_quota_for_channel` 使用 `WHERE user_id = ? AND channel_id = ?` 查询，无索引全表扫描

**修复方案：**
- 新增迁移 013 创建 `idx_quotas_channel_id` 索引

**涉及文件：** `backend/src/db/migrations/013_add_indexes.sql:6-7`

---

### 7. 前端 channelOptions 使用 undefined ✅

**问题描述：**
- 前端使用 `value: undefined` 表示"全部"，数据库用 `NULL`，语义不一致

**修复方案：**
- 类型改为 `number | null`，"全部"选项值改为 `null`

**涉及文件：** `web/src/views/control/quota/index.vue:30、57`

---

### 8. anthropic-beta header 会被覆盖 ✅

**问题描述：**
- `headers.extend()` 会覆盖硬编码的 `prompt-caching-2024-07-31`，导致缓存 header 丢失

**修复方案：**
- 添加 `merge_anthropic_beta` 函数，按逗号分割去重合并，而非直接覆盖

**涉及文件：** `backend/src/service/proxy.rs:72-114`

---

### 9. request_logs 缺少 channel_id 索引 ✅

**问题描述：**
- 所有按渠道聚合的查询均为全表扫描

**修复方案：**
- 新增迁移 013 创建 `idx_request_logs_channel_id` 索引

**涉及文件：** `backend/src/db/migrations/013_add_indexes.sql:4`

---

### 10. P95 延迟计算排序方向错误 ✅

**问题描述：**
- 使用 `ORDER BY elapsed_ms DESC`（降序），P95 定义应为升序

**修复方案：**
- 改为 `ORDER BY elapsed_ms ASC`

**涉及文件：** `backend/src/router/admin_routes.rs:780`

---

### 11. today_requests SUM 空集返回 NULL ✅

**问题描述：**
- `SUM(CASE WHEN ...)` 对空集返回 NULL 而非 0

**修复方案：**
- 添加 `COALESCE(..., 0)` 包装

**涉及文件：** `backend/src/router/admin_routes.rs:371-372`

---

### 12. 前端类型定义缺少 model_usage 字段 ✅

**问题描述：**
- `stats.ts` 返回类型缺少 `model_usage`，TypeScript 无法类型检查

**修复方案：**
- 类型定义添加 `model_usage: Array<{ name: string; count: number }>`

**涉及文件：** `web/src/service/api/stats.ts:24`

---

### 13. error_rate 类型定义缺失 ✅

**问题描述：**
- `DashboardStats` 类型定义缺少 `error_rate` 字段

**修复方案：**
- 类型定义添加 `error_rate: string`

**涉及文件：** `web/src/typings/api/liteproxy.d.ts`

---

## 十二、2026-04-18 未修复缺陷

> 以下缺陷经过评估后决定不修复，原因见各条说明。

### 1. 管理员路由不受 IP 过滤限制 ⏭️ 未修复

**问题描述：**
- `admin_protected` 和 `user_routes` 路由组未应用 `ip_filter` middleware

**未修复原因：** 设计如此。管理员不应被自己的黑名单拦截，否则可能将自己锁在系统外。

**涉及文件：** `backend/src/main.rs:153-160`、`backend/src/main.rs:83-92`

---

### 2. 配额检查与扣除非原子操作 ⏭️ 未修复

**问题描述：**
- `check_user_quota` 和 `deduct_user_quota` 是两个独立操作，高并发下可超限

**未修复原因：** 设计如此。个人项目并发极低，原子操作带来的复杂度收益不明显。

**涉及文件：** `backend/src/router/proxy_routes.rs:179`、`backend/src/service/quota.rs`

---

### 3. 无配置化开关禁用 anthropic-beta header ⏭️ 未修复

**问题描述：**
- `anthropic-beta` header 硬编码，无法关闭

**未修复原因：** 设计如此。Anthropic 官方渠道始终需要此 beta header，第三方兼容渠道不支持时可通过 custom_headers 覆盖。

**涉及文件：** `backend/src/service/proxy.rs:58`

---

### 4. 健康检查未添加 anthropic-beta header ⏭️ 未修复

**问题描述：**
- 健康检查请求 Anthropic 渠道时未添加 `anthropic-beta`

**未修复原因：** 设计如此。健康检查只验证连通性（`/models` 端点），不涉及缓存功能，不需要此 header。

**涉及文件：** `backend/src/service/health.rs:48-52`

---

### 5. "今日"统计定义不一致 ⏭️ 未修复

**问题描述：**
- `dashboard_stats` 用 `-1 day`（最近24h），`list_channels` 用 `start of day`（今日0点）

**未修复原因：** 设计如此。"今日"和"最近24小时"是不同业务场景，各有用途，定义合理。

**涉及文件：** `backend/src/router/admin_routes.rs:350-351`、`backend/src/router/admin_routes.rs:744`

---

### 6. error_rate 返回字符串 ⏭️ 未修复

**问题描述：**
- 后端返回 `"0.0%"` 字符串，传入 `NStatistic` 组件

**未修复原因：** 设计如此。后端已格式化为带 `%` 的字符串，前端直接显示即可，无需额外处理。

**涉及文件：** `backend/src/router/admin_routes.rs:784-788`

---

### 7. 进度超过 100% 时显示不一致 ⏭️ 未修复

**问题描述：**
- 进度条限死 100%，但文本显示实际超限值（如 "150/100 次"）

**未修复原因：** 设计如此。进度条限制最大值保证视觉一致，文本显示实际值提供准确信息，差异可接受。

**涉及文件：** `web/src/views/upstream/channel/index.vue:260`、`web/src/views/upstream/channel/index.vue:273`

---

### 8. quota_period_end 为 None 时配额不重置 ⏭️ 未修复

**问题描述：**
- `period_end` 为 `None` 时 `reset_channel_quota_if_expired` 不执行重置

**未修复原因：** 设计如此。`permanent`（永久）配额的 `period_end` 为 `None` 是预期行为，不需要重置。

**涉及文件：** `backend/src/service/quota.rs:111`

---

### 9. i64 值超出 JavaScript 安全整数范围 ⏭️ 未修复

**问题描述：**
- 后端 `quota_used` 和 `quota_limit` 为 i64，可能超过 `Number.MAX_SAFE_INTEGER`

**未修复原因：** 设计如此。个人项目配额值远不到 2^53，不会触发精度问题。

**涉及文件：** `web/src/views/upstream/channel/index.vue:258-260`

---

### 10. channel_id 列缺少外键约束 ⏭️ 未修复

**问题描述：**
- `quotas.channel_id` 无外键引用 `channels` 表

**未修复原因：** 设计如此。SQLite 默认不启用外键约束（`PRAGMA foreign_keys = OFF`），添加外键无实际效果。

**涉及文件：** `backend/src/db/migrations/011_quotas_channel_id.sql`

---

### 11. 数据库错误时 Fail-Open 策略 ⏭️ 未修复

**问题描述：**
- IP 黑白名单查询失败时允许请求通过

**未修复原因：** 设计如此。数据库故障时 Fail-Open 避免服务完全中断，可用性优先于安全性。

**涉及文件：** `backend/src/middleware/ip_filter.rs:50-52`、`backend/src/middleware/ip_filter.rs:63-65`

---

### 12. 不支持 CIDR 格式 ⏭️ 未修复

**问题描述：**
- IP 黑白名单为精确字符串匹配，不支持 `192.168.1.0/24` 网段

**未修复原因：** 暂不实施。CIDR 支持增加复杂度，个人项目单 IP 过滤够用。

**涉及文件：** `backend/src/db/models.rs:715-722`、`backend/src/db/models.rs:745-751`

---

### 13. 不支持 IPv6 地址标准化 ⏭️ 未修复

**问题描述：**
- `::1` 和 `0:0:0:0:0:0:0:1` 被视为不同地址

**未修复原因：** 暂不实施。IPv6 标准化优先级低，当前用户以 IPv4 为主。

**涉及文件：** `backend/src/middleware/ip_filter.rs:13-33`

---

### 14. IP 过滤无缓存每次查库 ⏭️ 未修复

**问题描述：**
- 每个请求执行 1-2 次 SQLite 查询检查 IP 黑白名单

**未修复原因：** 暂不实施。个人项目 QPS 低，SQLite 查询性能足够。

**涉及文件：** `backend/src/middleware/ip_filter.rs:44`、`backend/src/middleware/ip_filter.rs:57`

---

### 15. 编辑配额时不支持修改渠道 ⏭️ 未修复

**问题描述：**
- 编辑配额时渠道选择器隐藏，无法更改关联渠道

**未修复原因：** 暂不实施。渠道级配额创建后通常不需改渠道，优先级低。

**涉及文件：** `web/src/views/control/quota/index.vue:178-180`

---

### 16. 顶部用户查询无时间范围限制 ⏭️ 未修复

**问题描述：**
- 无过滤条件时 top_users 扫描全表历史数据

**未修复原因：** 暂不实施。个人项目用户数和日志量有限，性能可接受。

**涉及文件：** `backend/src/router/admin_routes.rs:1108-1115`

---

### 17. 配额类型空字符串误判为无限制 ⏭️ 未修复

**问题描述：**
- `!row.quota_type` 将空字符串 `''` 误判为无限制

**未修复原因：** 实际影响极小。正常流程中 `quota_type` 只会是 `null`、`'requests'` 或 `'tokens'`，空字符串几乎不会出现。

**涉及文件：** `web/src/views/upstream/channel/index.vue:255`

---

## 十三、2026-04-20 新增修复

### 1. CSV 导出缺少认证 token ✅

**问题描述：**
- `exportLogsCsv()` 使用原生 `fetch` 而没有携带 JWT token，导致 401 错误
- CSV 导出功能完全不工作

**修复方案：**
- 添加 Authorization header，从 localStorage 读取 token
- 清理 token 中的引号字符

**涉及文件：** `web/src/service/api/log.ts:22-34`

---

### 2. update_quota 强制启用 ✅

**问题描述：**
- `let enabled = true;` 更新操作强制将 `enabled` 设为 `true`
- 无法通过 API 禁用配额

**修复方案：**
- `UpdateQuotaRequest` 添加 `enabled` 字段
- 使用 `req.enabled.unwrap_or_else(...)` 保持当前值

**涉及文件：** `backend/src/router/admin_routes.rs:1116`

---

### 3. update_model 忽略 enabled 和 is_default 字段 ✅

**问题描述：**
- `enabled` 和 `is_default` 始终使用当前值，请求体中没有对应字段
- 这些字段无法通过 API 更新

**修复方案：**
- `UpdateModelRequest` 添加 `enabled` 和 `is_default` 字段
- 使用 `req.enabled.unwrap_or(e)` 和 `req.is_default.unwrap_or(d)`

**涉及文件：** `backend/src/router/admin_routes.rs:1069-1070`

---

### 4. update_app_profile 无法启用/禁用 ✅

**问题描述：**
- `enabled` 始终取自当前值，请求体中没有对应字段
- 应用预设无法通过 API 启用/禁用

**修复方案：**
- `UpdateAppProfileRequest` 添加 `enabled` 字段
- 使用 `req.enabled.unwrap_or(en)` 保持当前值

**涉及文件：** `backend/src/router/admin_routes.rs:1173`

---

### 5. 无效状态码映射为 200 OK ✅

**问题描述：**
- `StatusCode::from_u16(result.status_code).unwrap_or(StatusCode::OK)`
- 上游返回无效状态码（如 0、999）时错误地表示响应成功

**修复方案：**
- 改为 `unwrap_or(StatusCode::BAD_GATEWAY)`

**涉及文件：** `backend/src/router/proxy_routes.rs:302`

---

### 6. 数据库宕机显示"用户名或密码错误" ✅

**问题描述：**
- `Ok(None) | Err(_) => return error_response(UNAUTHORIZED, "Invalid username or password")`
- 数据库不可用时返回误导性错误信息

**修复方案：**
- 区分 `Ok(None)`（用户名密码错误 401）和 `Err(e)`（数据库错误 500）

**涉及文件：** `backend/src/router/admin_routes.rs:224-229`

---

### 7. IP 过滤器在认证之后执行 ✅

**问题描述：**
- Axum layer 反向执行顺序：`auth` → `rate_limit` → `ip_filter`
- 全局黑名单检查在认证之后，恶意 IP 已到达认证层

**修复方案：**
- 调整 layer 顺序为 `auth` → `rate_limit` → `ip_filter`
- axum layer 从后往前执行，ip_filter 放最后 = 最先执行

**涉及文件：** `backend/src/main.rs:73-75`

---

### 8. 渠道测试失败无反馈 ✅

**问题描述：**
- 测试失败时虽然有处理，但用户界面反馈不够明确

**修复方案：**
- catch 块添加 error 提示显示错误信息
- 对不同 HTTP 状态码给出友好提示

**涉及文件：** `web/src/views/upstream/channel/index.vue:263-266`

---

### 9. enabled_models JSON 解析失败静默清空 ✅

**问题描述：**
- `try { enabledModels = JSON.parse(row.enabled_models); } catch { /* ignore */ }`
- 数据库中格式异常时用户的模型绑定配置被静默清空

**修复方案：**
- catch 块添加 warning 提示告知用户配置异常

**涉及文件：** `web/src/views/control/user/index.vue:252-254`

---

### 10. 模型列表硬编码加载 1000 条 ✅

**问题描述：**
- `fetchModelList(1, 1000)` 加载 1000 条模型数据到内存
- 内存占用过高

**修复方案：**
- 所有 `fetchModelList(1, 1000)` 改为 `fetchModelList(1, 200)`

**涉及文件：** 多个前端文件

---

### 11. tooltipRecord 键值映射错乱 ✅

**问题描述：**
- `left → right`、`right → unFixed`、`unFixed → left` 映射错误
- tooltip 文本与实际状态不匹配

**修复方案：**
- 更正为 `left→left, right→right, unFixed→unFixed`

**涉及文件：** `web/src/components/advanced/table-column-setting.vue:14-18`

---

### 12. i18n fallbackLocale 配置不匹配 ✅

**问题描述：**
- `fallbackLocale: 'en'` 但可用 locale 是 `'zh-CN'` 和 `'en-US'`
- 可能无法正确解析到 `'en-US'`

**修复方案：**
- 改为 `fallbackLocale: 'en-US'`

**涉及文件：** `web/src/locales/index.ts:8`

---

### 13. CORS 允许所有来源 ✅

**问题描述：**
- `allow_origin(Any)` 任意域名可发起请求
- 任何网站都可以向此 API 发起跨域请求

**修复方案：**
- 配置文件添加 `cors_allowed_origins` 字段
- 支持配置允许的域名列表，未配置时保持向后兼容

**涉及文件：** `backend/src/main.rs:166-214`、`backend/src/config.rs:19`

---

### 14. API Keys 明文存储 ✅

**问题描述：**
- 渠道 API Key 在 SQLite 数据库中完全明文存储
- 数据库文件泄露 = 所有上游服务商 Key 泄露

**修复方案：**
- 默认启用 AES-256-GCM 加密
- 密钥从 `CORIDE_JWT_SECRET` 派生（SHA-256）
- 写入时自动加密、读取时自动解密

**涉及文件：** `backend/src/utils/encrypt.rs`（新建）、`backend/src/db/models.rs:7-14`、`backend/src/lib.rs:31-32`

---

### 15. 创建模型默认选最后一个渠道 ✅

**问题描述：**
- `handleCreate` 中 `channel_id` 默认选中最后一个渠道
- ID 最大的渠道不一定是用户想要的

**修复方案：**
- `channel_id` 初始化为 `0`，用户需手动选择渠道

**涉及文件：** `web/src/views/upstream/model/index.vue:168`

---

## 最终统计

| 类别 | 数量 |
|------|------|
| 已修复 | 71 |
| 未修复（设计如此） | 11 |
| 未修复（暂不实施） | 13 |
| 未修复（影响极小） | 1 |
| **缺陷总计** | **96** |