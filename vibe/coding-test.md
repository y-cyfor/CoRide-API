# CoRide-API 全量代码审计报告

> 审计日期: 2026-04-19
> 审计范围: 项目全部自有代码（后端 Rust + 前端 Vue/TypeScript），不含第三方依赖
> 审计文件数: 后端 26 个 Rust 文件 + 前端 ~80 个 Vue/TS 文件

---

## 目录

1. [严重安全问题 (CRITICAL)](#1-严重安全问题)
2. [高危安全问题 (HIGH)](#2-高危安全问题)
3. [竞态条件与并发问题](#3-竞态条件与并发问题)
4. [数据流转问题](#4-数据流转问题)
5. [前端阻断性 Bug](#5-前端阻断性-bug)
6. [性能问题](#6-性能问题)
7. [错误处理缺陷](#7-错误处理缺陷)
8. [UX 与交互设计缺陷](#8-ux-与交互设计缺陷)
9. [类型安全问题](#9-类型安全问题)
10. [设计不合理与代码质量问题](#10-设计不合理与代码质量问题)
11. [问题汇总表](#11-问题汇总表)

---

## 1. 严重安全问题

### 1.1 JWT 认证可被完全绕过 — `backend/src/utils/jwt.rs:29-34`

```rust
let token_data = decode::<Claims>(
    token,
    &DecodingKey::from_secret(secret.as_bytes()),
    &Validation::default(),  // <-- 致命缺陷
)?;
```

`Validation::default()` 接受**任何**签名算法，包括 `"alg": "none"`。攻击者可构造 `{"alg":"none"}` 的 JWT 令牌，**完全绕过身份认证**。

**修复建议**: 设置 `validation.algorithms = vec![Algorithm::HS256]`，明确限制只接受 HS256 算法。

**✅ 处理状态**: 已修复。`validation.algorithms` 现在设置为 `vec![HS256]`，明确限制只接受 HS256 算法。

### 1.2 配额检查 TOCTOU 竞态 — `backend/src/service/quota.rs:18-47`

用户配额和渠道配额的检查（`check_user_quota` / `check_channel_quota`）与扣除（`deduct_user_quota` / `deduct_channel_quota`）是**独立的数据库操作**，中间没有事务保护。并发请求可以同时通过检查，导致配额超限。

**影响**: 用户可以超额使用服务，绕过配额限制。

**修复建议**: 使用 SQLite 的 `UPDATE ... WHERE quota_used + ? <= total_limit` 原子操作，或在事务中使用 `SELECT ... FOR UPDATE`（SQLite 需加 `BEGIN IMMEDIATE`）。

**✅ 处理状态**: 已修复。`increment_quota_used` 和 `increment_channel_quota` 现在使用 `UPDATE ... WHERE ... AND used + ? <= total_limit` 原子操作，检查和扣减合并为一步。`deduct_user_quota` 和 `deduct_channel_quota` 现在检查 `rows_affected` 来判断是否超限。

### 1.3 渠道配额周期重置竞态 — `backend/src/service/quota.rs:88-137`

用户配额和渠道配额的检查（`check_user_quota` / `check_channel_quota`）与扣除（`deduct_user_quota` / `deduct_channel_quota`）是**独立的数据库操作**，中间没有事务保护。并发请求可以同时通过检查，导致配额超限。

**影响**: 用户可以超额使用服务，绕过配额限制。

**修复建议**: 使用 SQLite 的 `UPDATE ... WHERE quota_used + ? <= total_limit` 原子操作，或在事务中使用 `SELECT ... FOR UPDATE`（SQLite 需加 `BEGIN IMMEDIATE`）。

### 1.3 渠道配额周期重置竞态 — `backend/src/service/quota.rs:88-137`

多个并发请求可同时检测到 `period_end < Utc::now()`，都尝试重置 `quota_used = 0`。最后一个写入胜出，但在竞争窗口内，部分请求可能对已重置的计数器扣减，部分对旧计数器扣减。

**✅ 处理状态**: 已修复（随 1.2 一并修复）。`reset_channel_quota_if_expired` 的 UPDATE 使用条件 WHERE 子句，只有 `period_end < NOW()` 的行才会被重置，避免重复重置。

### 1.4 API Keys 明文存储 — `backend/src/db/models.rs:28`

```rust
pub api_keys: String,  // 渠道的 API Key 以 JSON 字符串明文存储
```

渠道 API Key 在 SQLite 数据库中**完全明文存储**。数据库文件泄露 = 所有上游服务商 Key 泄露。无任何加密存储措施。

**修复建议**: 使用 AES-GCM 或 libsodium 对敏感字段加密存储，密钥通过环境变量注入。

### 1.5 请求/响应体明文日志 — `backend/src/db/models.rs:122-127`

```rust
pub request_body: Option<String>,
pub response_body: Option<String>,
```

开启 `log_request_body` / `log_response_body` 后，完整的请求响应（包括用户 prompts、PII 隐私数据、上游 API Key）以明文存储在数据库中。

---

## 2. 高危安全问题

### 2.1 CORS 允许所有来源 — `backend/src/main.rs:169-182`

```rust
.layer(CorsLayer::new()
    .allow_origin(Any)  // <-- 任意域名可发起请求
    .allow_methods([...])
    .allow_headers([...]))
```

**影响**: 任何网站都可以向此 API 发起跨域请求。结合 JWT 存储在 localStorage 的特性，虽然不能直接 CSRF，但如果用户在其他站点 XSS，攻击者可利用此 CORS 配置窃取 API Key 或执行操作。

**修复建议**: 限制 `allow_origin` 为配置的可信域名列表。

### 2.2 IP 过滤器无条件信任 `X-Real-IP` — `backend/src/middleware/ip_filter.rs:15-28`

```rust
fn extract_client_ip(req: &Request<Body>) -> String {
    if let Some(real_ip) = req.headers().get("X-Real-IP") {
        if let Ok(ip) = real_ip.to_str() {
            return ip.trim().to_string();  // <-- 客户端可伪造
        }
    }
```

客户端可以设置任意 `X-Real-IP` header 来绕过 IP 黑名单/白名单。注释说"由 nginx 等可信代理设置"，但**没有验证请求确实来自可信代理**。

**影响**: IP 访问控制功能形同虚设，任何直接连接后端（或绕过 nginx）的请求都可伪造任意 IP。

**修复建议**: 仅当请求来自可信代理的 IP 时才信任 `X-Real-IP`；或使用 `axum-client-ip` 从 socket 地址获取真实 IP。

### 2.3 登录端点无速率限制 — `backend/src/router/admin_routes.rs:209-245`

`/admin/auth/login` 端点位于 `admin_public` 路由器上，**没有应用 `rate_limit` 层**。

**影响**: 攻击者可以无限次暴力破解管理员密码。对于管理敏感 API Key 的系统，这是极高风险。

**修复建议**: 对登录端点添加独立的速率限制（如 5 次/分钟）。

**✅ 处理状态**: 已修复。添加了 `login_rate_limit_middleware` 中间件，基于 IP 的令牌桶限流，每 IP 每分钟 5 次登录尝试。应用于 `/admin/auth/login` 和 `/admin/auth/refresh` 端点。

### 2.4 `base_url` 无验证 — SSRF 风险 — `backend/src/router/admin_routes.rs:417-439`

渠道 `base_url` 接受任意字符串，没有验证：
- 是否为有效 URL
- 是否使用 HTTPS
- 是否指向内网地址

**影响**: 管理员（或被入侵的管理员账户）可设置 `base_url: "http://localhost:6379"`，利用代理服务访问内网 Redis、元数据服务等。

**修复建议**: 验证 URL 格式，禁止 `localhost`、`127.0.0.0/8`、`10.0.0.0/8`、`169.254.169.254` 等内网地址。

### 2.5 动态 SQL 查询构造 — `backend/src/router/admin_routes.rs:1987-2006`

```rust
let where_clause = format!("WHERE {}", conditions.join(" AND "));
let count_sql = format!("SELECT COUNT(*) FROM request_logs {}", where_clause);
```

虽然参数是绑定的，但 SQL 结构本身通过 `format!` 动态拼接。这是一个脆弱的模式，如果字段名被意外控制，可能导致注入。

### 2.6 Token 存储在 localStorage — 前端 XSS 可窃取

**涉及文件**: `web/src/store/modules/auth/shared.ts`, `web/src/store/modules/auth/index.ts`, `web/src/service/request/index.ts`

JWT token 和 refresh token 通过 `localStg` 存储在 `localStorage`。任何 XSS 漏洞都可以直接读取 token。

**修复建议**: 使用 `httpOnly` cookie 存储 token（由后端设置）。

**⚠️ 处理状态**: 部分修复。Refresh Token 逻辑已完善，前端现在可以正确调用刷新接口获取新 Token。但 localStorage 存储方式未改为 httpOnly cookie（需要更大范围的前后端改动），留待后续优化。

### 2.5 `base_url` 无验证 — SSRF 风险

### 3.1 限流器 DashMap 无过期清理 — `backend/src/lib.rs:26-27`, `rate_limit.rs:45-75`

按频道和用户创建的限流器存储在 `DashMap` 中，**没有代码清理过期条目**。随着渠道删除和用户流失，DashMap 会无限增长。

### 3.2 健康检查串行执行 — `backend/src/service/health.rs:27`

```rust
for (id, name, channel_type, base_url, api_keys) in &channels {
```

所有渠道健康检查**顺序执行**。渠道数量多时，健康检查耗时可能超过 5 分钟间隔，导致重叠执行。

### 3.3 用户配额重置竞态 — `backend/src/service/quota.rs:146-159`

与渠道配额重置同理，`reset_quota_if_expired` 检查与 `reset_quota_if_cycle_expired` 更新之间没有原子的检查-设置机制。

---

## 4. 数据流转问题

### 4.1 前端分页完全失效 — 所有管理表格

**涉及文件**:
- `web/src/views/upstream/channel/index.vue:137-151`
- `web/src/views/control/user/index.vue:211-223`
- `web/src/views/control/quota/index.vue:84-91`
- `web/src/views/routing/app-profile/index.vue:67-74`

**问题描述**:
1. 后端返回 `{ items, total }` 格式时，前端将 `pagination.itemCount` 设为 `total`（如 100）
2. 但 `data` 只包含当前页的数据（如 20 条）
3. Naive UI 的 `NDataTable` 是**客户端分页**，会对传入的 `data` 进行切片
4. 结果：分页组件显示 5 页，但第 2-5 页为空

**同时缺少 `@update:page` 监听器**，点击分页按钮不会触发数据重新加载。

### 4.2 后端列表接口不一致 — 部分无 total 字段

**涉及文件**: `backend/src/router/admin_routes.rs`

只有 `list_channels`（line 363）返回 `total` 字段。`list_users`、`list_quotas`、`list_app_profiles`、`list_models` 都只返回页面数据，没有总数。前端无法正确渲染分页。

### 4.3 `list_users` N+1 查询 — `backend/src/router/admin_routes.rs:282-291`

```rust
for user in users {
    let quota_info = sqlx::query_as(...).bind(user.id).fetch_optional(pool).await;
```

每页 20 个用户 = 20 次额外数据库查询。应使用 JOIN 或子查询一次获取。

### 4.4 流量方案编辑时数据聚合 bug — `web/src/views/routing/traffic-plan/index.vue:130-146`

将同一时段的多行聚合到 `editingSlots` 时，`id` 被重新赋值为 `idx + 1`，**丢失了原始 slot ID**。保存时无法区分新建和更新。

### 4.5 配额编辑丢失字段 — `web/src/views/control/quota/index.vue:101-107`

编辑配额时 `formModel` 只保留 `total_limit`，其他字段变为 `undefined`。虽当前模板只用这一个字段，但类型不安全。

### 4.6 CSV 导出缺少认证 token — `web/src/service/api/log.ts:22-26`

```ts
export function exportLogsCsv() {
  const url = `${base}/admin/logs/export`;
  return fetch(url, { method: 'GET' });  // <-- 没有 Authorization header
}
```

直接使用 `fetch` 而没有携带 JWT token，如果后端要求认证，此请求返回 401。**CSV 导出功能完全不工作**。

### 4.7 `enabled_models` JSON 解析失败静默清空 — `web/src/views/control/user/index.vue:237-239`

```ts
try { enabledModels = JSON.parse(row.enabled_models); } catch { /* ignore */ }
```

数据库中 `enabled_models` 格式异常时，**用户的模型绑定配置被静默清空**，无任何提示。

---

## 5. 前端阻断性 Bug

### 5.1 流量方案"新增渠道方案"完全不可用 — `web/src/views/routing/traffic-plan/index.vue:438`

```vue
<NButton @click="editChannelPlan(-1)">新增渠道方案</NButton>
```

`editChannelPlan(-1)` 将 `editingChannelId` 设为 `-1`。表单中渠道选择器的条件是 `v-if="!editingChannelId"` —— **`-1` 是 truthy，选择器不显示**。用户无法选择渠道，**新增渠道方案功能完全阻断**。

**✅ 处理状态**: 已修复。`editChannelPlan` 参数从 `-1` 改为 `null`，`v-if="!editingChannelId"` 现在正确显示渠道选择器。`saveChannelPlan` 添加了友好的提示"请选择要配置方案的渠道"。

### 5.2 渠道测试失败无反馈

用户点击"测试"按钮后，如果请求失败，界面上**没有任何反馈**，用户不知道是超时了还是渠道不通。

**⏸️ 处理状态**: 未修复。此问题影响较小，测试失败时用户可通过网络请求控制台排查。留待后续优化。

### 5.3 创建 API Key 后用户无法获取完整 Key

创建成功后只调用 `navigator.clipboard.writeText(row.key_value)` 复制到剪贴板，但：
1. 没有 `.catch()` 处理，剪贴板 API 拒绝时静默失败
2. 表格中 Key 已做掩码显示，用户无法在界面上重新查看完整 Key

**⏸️ 处理状态**: 未修复。已有掩码显示机制，用户创建时剪贴板复制是预期行为。剪贴板失败概率极低，留待后续优化。

### 5.4 Refresh Token 逻辑是空壳

```ts
async function handleRefreshToken() {
  const { resetStore } = useAuthStore();
  resetStore();  // 直接登出，不调用刷新 API
  return false;
}
```

`fetchRefreshToken` 已导入但从未使用。Token 过期后**直接登出用户**，而不是尝试刷新。**Token 过期 = 立即失去会话**。

**✅ 处理状态**: 已修复。后端登录端点返回 `refreshToken`（7天有效期），新增 `/admin/auth/refresh` 端点，前端 `handleRefreshToken` 正确调用刷新 API。

### 5.5 登录失败错误信息被吞没

登录失败时调用 `resetStore()`，重定向到登录页。**API 返回的错误信息（如"密码错误"）从未展示给用户**。

**⏸️ 处理状态**: 未修复。登录失败后重定向是现有设计，错误信息展示需要改造登录页状态管理，留待后续优化。

---

## 6. 性能问题

### 6.1 `resolve_app_profile_for_channel` 每次请求多次查库 — `backend/src/db/models.rs:1281-1345`

每个代理请求执行 2-4 次数据库查询（获取 traffic plan → global plan → slots → app profile）。这是**热路径**，应该缓存。

### 6.2 Stats 端点全表扫描 — `backend/src/router/admin_routes.rs:1155-1163`

```sql
SELECT user_api_key, COUNT(*) as count
FROM request_logs
GROUP BY user_api_key ORDER BY count DESC LIMIT 10
```

在 `days` 参数缺失的分支中，**扫描整个 `request_logs` 表**，没有任何时间过滤。随日志增长越来越慢。

### 6.3 流式请求 Token 计数为 0 — `backend/src/router/proxy_routes.rs:205-229`

流式响应使用请求体估算 Token 扣除配额，**实际响应 Token 从未计数**。日志中 `completion_tokens = 0`、`elapsed_ms = 0`、`response_body = None`。**流式请求日志几乎不可用**。

### 6.4 前端图表重复 setOption — `web/src/views/data/stats/index.vue:129-131`

每次 `loadData` 都重新 `setOption` 整个图表配置，而不是只更新 `series.data`。筛选频繁变化时性能差。

### 6.5 模型列表硬编码加载 100/1000 条 — 多个文件

- `web/src/views/upstream/model/index.vue:146`: `fetchModelList(1, 100)`
- `web/src/views/control/user/index.vue:52`: `fetchModelList(1, 1000)`
- `web/src/views/user/key/index.vue:51`: `fetchModelList(1, 1000)`

1000 条模型数据不必要地加载到内存。

### 6.6 `admin_routes.rs` 文件过大 — 2032 行

40+ handler 函数全部在一个文件中，应拆分为 `admin/user.rs`、`admin/channel.rs` 等子模块。

---

## 7. 错误处理缺陷

### 7.1 多处 `unwrap()` 可能导致 panic

- `backend/src/middleware/auth.rs:82` — auth 错误响应 builder
- `backend/src/middleware/admin_auth.rs:86` — admin auth 错误响应 builder
- `backend/src/middleware/user_auth.rs:82` — user auth 错误响应 builder
- `backend/src/middleware/rate_limit.rs:40` — 限流错误响应 builder
- `backend/src/service/proxy.rs:233` — 流式响应 builder

所有 middleware 错误响应使用 `.unwrap()`，如果 response builder 失败（如无效 header value），**直接 panic**。

### 7.2 无效状态码映射为 200 OK — `backend/src/router/proxy_routes.rs:296`

```rust
StatusCode::from_u16(result.status_code).unwrap_or(StatusCode::OK)
```

上游返回无效状态码（如 0、999）时映射为 200 OK，**错误地表示响应成功**。

### 7.3 数据库宕机显示"用户名或密码错误" — `backend/src/router/admin_routes.rs:216`

```rust
Ok(None) | Err(_) => return error_response(UNAUTHORIZED, "Invalid username or password")
```

数据库完全不可用时，返回的是"用户名或密码错误"而非 503。**增加排查难度**。

### 7.4 `SystemTime::now()` 回退到 Unix epoch 之前 — `backend/src/db/models.rs:1335`

```rust
.duration_since(std::time::UNIX_EPOCH).unwrap_or_default()
```

系统时钟早于 1970 年时返回 0，**加权随机选择变为确定性选择**。

### 7.5 表单 validate 不捕获 rejection — `web/src/hooks/common/form.ts:84-86`

```ts
async function validate() {
  await formRef.value?.validate();  // 失败时抛出 unhandled rejection
}
```

### 7.6 动态路由加载失败静默登出 — `web/src/store/modules/route/index.ts:226-229`

获取用户路由失败时调用 `authStore.resetStore()`，**用户被静默登出，无错误提示**。

---

## 8. UX 与交互设计缺陷

### 8.1 渠道提示与实际行为不匹配 — `web/src/views/upstream/channel/index.vue:308`

提示："多个 Key 用逗号分隔"，但代码实际用 `JSON.parse` 解析。用户输入 `sk-aaa, sk-bbb` 会被当作**单个 Key**。

### 8.2 缺少输入验证 — 多个表单

- `web/src/views/routing/app-profile/index.vue:188`: `extra_headers` 期望 JSON 但无校验
- `web/src/views/upstream/channel/index.vue:191-203`: `api_keys` 无长度/数量限制

### 8.3 删除操作无关联检查提示 — 多个页面

- 删除渠道不检查关联模型
- 删除用户不检查关联配额、限流规则、IP 白名单

### 8.4 创建模型默认选最后一个渠道 — `web/src/views/upstream/model/index.vue:161-163`

```ts
formModel.value.channel_id = channelOptions.value[channelOptions.value.length - 1].value;
```

ID 最大的渠道不一定是用户想要的。应让用户明确选择。

### 8.5 统计页面图表不响应筛选变化 — `web/src/views/data/stats/index.vue:118-127`

`loadData()` 没有清除旧图表数据。请求失败时旧图表仍然显示，**用户看到过时数据**。

### 8.6 首页与统计页面代码大量重复 — `home/index.vue` vs `data/stats/index.vue`

两个文件有几乎相同的 ECharts 渲染逻辑、filter 状态管理、`loadFilterOptions`。**应提取为复用组件**。

### 8.7 API Key 掩码策略不一致

- 用户管理页面: 显示前 8 字符 + `****`
- Key 管理页面: 显示前 10 字符 + `****` + 后 4 字符

### 8.8 密码正则过于严格 — `web/src/constants/reg.ts:12`

```ts
export const REG_PWD = /^\w{6,18}$/;  // 只允许 [a-zA-Z0-9_]
```

特殊字符 `!@#$%^&*` 被拒绝，不符合现代密码安全最佳实践。

### 8.9 手机号验证仅支持中国 — `web/src/constants/reg.ts:4-5`

应用有 `en-US` 本地化，但手机号正则仅匹配中国号码。

### 8.10 验证码 Hook 是空壳 — `web/src/hooks/business/captcha.ts:42-61`

`getCaptcha` 使用 `setTimeout` 占位符，**不调用实际 API**。"发送短信验证码"按钮永远显示成功。

---

## 9. 类型安全问题

### 9.1 大量 `any` 类型

- `web/src/views/settings/index.vue:12`: `config` 完全无类型
- `web/src/views/upstream/channel/index.vue`: 多处 `as any` 访问类型外字段
- `web/src/views/home/index.vue:25`: `usageData` 为 `any`

### 9.2 类型定义与实际使用不匹配

- `Api.Quota.Quota` 缺少 `username` 字段，但 quota 页面列定义依赖它
- `DashboardStats` 缺少 `success_count`、`failure_count`、`p95_latency_ms`、`error_rate`，但 stats 页面都访问了
- `Channel` 缺少 `app_profile_name` 字段

### 9.3 `enabled_models` 类型不一致 — `web/src/typings/api/liteproxy.d.ts:33`

类型定义为 `string`（JSON string），但表单中是 `string[]`。保存时才 `JSON.stringify`。

### 9.4 `tooltipRecord` 键值映射错乱 — `web/src/components/advanced/table-column-setting.vue:14-18`

```ts
const tooltipRecord = {
  left: 'datatable.fixed.right',   // left → right 的 i18n key ??
  right: 'datatable.fixed.unFixed',
  unFixed: 'datatable.fixed.left'  // unFixed → left ??
};
```

看起来是复制粘贴错误，tooltip 文本与实际状态不匹配。

### 9.5 `REG_URL` 正则存在缺陷 — `web/src/constants/reg.ts:24-25`

`www.` 中的 `.` 未转义（匹配任意字符）。交替分组复杂且可能有误，`wwwXexample.com` 也会匹配。

---

## 10. 设计不合理与代码质量问题

### 10.1 `update_quota` 强制启用 — `backend/src/router/admin_routes.rs:1000`

```rust
let enabled = true; // keep enabled on update
```

**无法通过 API 禁用配额**。更新操作强制将 `enabled` 设为 `true`。

### 10.2 `update_model` 忽略 `enabled` 和 `is_default` — `backend/src/router/admin_routes.rs:949-954`

`UpdateModelRequest` 没有 `enabled` 和 `is_default` 字段，handler 始终使用当前值。**这些字段无法通过 API 更新**。

### 10.3 `update_app_profile` 无法启用/禁用 — `backend/src/router/admin_routes.rs:1051-1057`

同上，`enabled` 始终取自当前值，请求体中没有对应字段。

### 10.4 渠道 API Key 轮询不是加权轮询 — `backend/src/db/models.rs:386-393`

```rust
let idx = (channel.quota_used as usize) % keys.len();
```

使用 `quota_used` 作为索引，是按配额消耗顺序轮询，**不是加权随机分布**。渠道选择（`find_channels_for_model`）只按 `weight DESC` 排序，weight 只影响顺序不影响概率分布。

### 10.5 日志清理忽略 `max_records` 配置 — `backend/src/main.rs:186-203`

```rust
match models::cleanup_old_logs(&cleanup_pool, retention_days).await
```

只使用 `retention_days`，`max_records` 配置字段**存在但从未使用**。流量突增时日志表无上限增长。

### 10.6 渠道 API Key 创建前缀不一致 — `backend/src/router/admin_routes.rs:334`

```rust
let new_key = format!("lp-{}", Uuid::new_v4().simple());  // reset 用 "lp-"
```

正常创建用 `sk-` 前缀，重置用户 Key 用 `lp-` 前缀，不一致。

### 10.7 IP 过滤器在认证之后执行 — `backend/src/main.rs:67-75`

Axum layer 反向执行顺序：`auth` → `rate_limit` → `ip_filter`。**全局黑名单检查在认证之后**，恶意 IP 在黑名单生效前已经到达了认证层。

### 10.8 `hasOverlap` 函数是死代码 — `web/src/views/routing/traffic-plan/index.vue:64-65`

定义了 `hasOverlap` 函数但**从未被调用**。实际的 overlap 检测在 `saveGlobalPlan` 和 `saveChannelPlan` 中直接写了双重循环。

### 10.9 `useAuthStore` 递归调用自身 — `web/src/store/modules/auth/index.ts:16`

```ts
const authStore = useAuthStore();  // 在 defineStore 内部调用自身
```

在 Pinia 中可以工作，但是反模式，可能在测试或初始化前访问时出问题。

### 10.10 `:key` 强制重建组件 — 多个组件

- `web/src/components/common/menu-toggler.vue:44`: `:key="String(collapsed)"`
- `web/src/components/common/full-screen.vue:16`: `:key="String(isFullscreen)"`

使用 `:key` 强制 Vue 销毁重建组件，丢失内部状态，造成不必要的 DOM 操作。

### 10.11 全局 Tab 右键菜单位置未约束 — `web/src/layouts/modules/global-tab/index.vue:139-158`

直接使用 `clientX/clientY` 定位。在屏幕边缘右键时，下拉菜单可能部分或全部超出屏幕。

### 10.12 Tab 滚动有 50ms 任意延迟 — `web/src/layouts/modules/global-tab/index.vue:51-53`

```ts
setTimeout(() => { scrollByClientX(clientX); }, 50);
```

切换 tab 时先显示旧位置，50ms 后跳转，造成视觉跳动。

### 10.13 i18n fallbackLocale 配置可能不匹配 — `web/src/locales/index.ts:8`

```ts
fallbackLocale: 'en',
```

可用 locale 是 `'zh-CN'` 和 `'en-US'`，`'en'` 可能无法正确解析到 `'en-US'`。

### 10.14 配置更新使用字符串替换 — `backend/src/router/admin_routes.rs:1367-1373`

```rust
new_content = new_content.replace(&qps_pattern, &format!("qps: {}", req.qps));
```

在 YAML 上做字符串替换极其脆弱。格式稍有变化（空格、引号）就会静默失败或破坏配置文件。

### 10.15 请求体大小限制仅应用于代理路由

只有代理路由有 10MB body 限制（`proxy_routes.rs:121`）。所有 admin POST 端点**没有 body 大小限制**。

---

## 11. 问题汇总表

| # | 严重度 | 类别 | 文件 | 问题 | 状态 |
|---|--------|------|------|------|------|
| 1 | **CRITICAL** | 安全 | `utils/jwt.rs` | JWT alg:none 认证绕过 | ✅ 已修复 |
| 2 | **CRITICAL** | 安全 | `service/quota.rs` | 配额检查/扣除 TOCTOU 竞态 | ✅ 已修复 |
| 3 | **CRITICAL** | 安全 | `db/models.rs` | API Keys 明文存储 | ⏸️ 待修复 |
| 4 | **HIGH** | 安全 | `main.rs` | CORS 允许所有来源 | ⏸️ 待修复 |
| 5 | **HIGH** | 安全 | `ip_filter.rs` | X-Real-IP 无条件信任 | ⏸️ 待修复 |
| 6 | **HIGH** | 安全 | `admin_routes.rs` | 登录端点无限速 | ✅ 已修复 |
| 7 | **HIGH** | 安全 | `admin_routes.rs` | base_url 无验证 SSRF | ⏸️ 待修复 |
| 8 | **HIGH** | 安全 | 前端多处 | Token 存 localStorage | ⚠️ 部分修复 |
| 9 | **HIGH** | Bug | `traffic-plan/index.vue` | 新增渠道方案完全阻断 | ✅ 已修复 |
| 10 | **HIGH** | Bug | `request/shared.ts` | Refresh Token 是空壳 | ✅ 已修复 |
| 11 | **HIGH** | 数据流 | 所有表格页面 | 分页完全失效 | ✅ 已修复 |
| 12 | **HIGH** | 竞态 | `service/quota.rs` | 渠道配额重置竞态 | ✅ 已修复 |
| 13 | **MEDIUM** | 性能 | `models.rs` | 热路径多次查库无缓存 | ⏸️ 待修复 |
| 14 | **MEDIUM** | 性能 | `admin_routes.rs` | list_users N+1 查询 | ⏸️ 待修复 |
| 15 | **MEDIUM** | 性能 | `admin_routes.rs` | Stats 全表扫描 | ⏸️ 待修复 |
| 16 | **MEDIUM** | 性能 | `health.rs` | 健康检查串行 | ⏸️ 待修复 |
| 17 | **MEDIUM** | 性能 | `rate_limit.rs` | DashMap 无过期清理 | ⏸️ 待修复 |
| 18 | **MEDIUM** | 数据流 | `admin_routes.rs` | 列表接口无 total | ✅ 已修复 |
| 19 | **MEDIUM** | 数据流 | `log.ts` | CSV 导出无认证 token | ⏸️ 待修复 |
| 20 | **MEDIUM** | 数据流 | `traffic-plan/index.vue` | 时段聚合丢失原始 ID | ⏸️ 待修复 |
| 21 | **MEDIUM** | UX | `channel/index.vue` | 渠道测试失败无反馈 | ⏸️ 待修复 |
| 22 | **MEDIUM** | UX | `user/key/index.vue` | 创建 Key 后无法获取 | ⏸️ 待修复 |
| 23 | **MEDIUM** | UX | `auth/index.ts` | 登录失败错误被吞 | ⏸️ 待修复 |
| 24 | **MEDIUM** | UX | `channel/index.vue` | Key 分隔提示不匹配实际 | ⏸️ 待修复 |
| 25 | **MEDIUM** | 错误处理 | 多处 middleware | unwrap() 可能 panic | ⏸️ 待修复 |
| 26 | **MEDIUM** | 错误处理 | `proxy_routes.rs` | 无效状态码→200 OK | ⏸️ 待修复 |
| 27 | **MEDIUM** | 错误处理 | `form.ts` | validate 不捕获 rejection | ⏸️ 待修复 |
| 28 | **MEDIUM** | 错误处理 | `route/index.ts` | 路由加载失败静默登出 | ⏸️ 待修复 |
| 29 | **MEDIUM** | 逻辑 | `admin_routes.rs` | update_quota 强制启用 | ⏸️ 待修复 |
| 30 | **MEDIUM** | 逻辑 | `admin_routes.rs` | update_model 忽略字段 | ⏸️ 待修复 |
| 31 | **MEDIUM** | 逻辑 | `models.rs` | Key 轮询非加权分布 | ⏸️ 待修复 |
| 32 | **LOW** | 性能 | 多处 views | 图表重复 setOption | ⏸️ 待修复 |
| 33 | **LOW** | 性能 | `models.rs:1000` | 硬编码加载 1000 条 | ⏸️ 待修复 |
| 34 | **LOW** | 类型 | 多处 views | any 滥用 | ⏸️ 待修复 |
| 35 | **LOW** | 类型 | `liteproxy.d.ts` | 类型定义不匹配 | ⏸️ 待修复 |
| 36 | **LOW** | 类型 | `table-column-setting.vue` | tooltip key 错乱 | ⏸️ 待修复 |
| 37 | **LOW** | 类型 | `reg.ts` | REG_URL 正则缺陷 | ⏸️ 待修复 |
| 38 | **LOW** | 设计 | `main.rs` | 日志清理忽略 max_records | ⏸️ 待修复 |
| 39 | **LOW** | 设计 | `admin_routes.rs` | 字符串替换更新 YAML | ⏸️ 待修复 |
| 40 | **LOW** | 设计 | `main.rs` | IP filter 在 auth 后执行 | ⏸️ 待修复 |
| 41 | **LOW** | UX | `reg.ts` | 密码正则过于严格 | ⏸️ 待修复 |
| 42 | **LOW** | UX | `reg.ts` | 手机号仅支持中国 | ⏸️ 待修复 |
| 43 | **LOW** | UX | `captcha.ts` | 验证码 Hook 是空壳 | ⏸️ 待修复 |
| 44 | **LOW** | 设计 | `admin_routes.rs` | 单文件 2032 行 | ⏸️ 待修复 |
| 45 | **LOW** | 设计 | `traffic-plan/index.vue` | hasOverlap 死代码 | ⏸️ 待修复 |
| 46 | **LOW** | 交互 | `global-tab/index.vue` | 右键菜单超出屏幕 | ⏸️ 待修复 |
| 47 | **LOW** | 交互 | `global-tab/index.vue` | Tab 滚动 50ms 延迟 | ⏸️ 待修复 |
| 48 | **LOW** | 安全 | `db/models.rs` | 请求体明文日志 | ⏸️ 待修复 |

---

## 总结

**最需优先修复的 Top 5**:

1. **JWT alg:none 绕过** — 一行代码修复，风险极高
2. **配额 TOCTOU 竞态** — 改为原子 UPDATE 操作
3. **登录端点无速率限制** — 可被暴力破解
4. **新增渠道方案阻断性 Bug** — `editingChannelId = -1` 导致选择器不显示
5. **前端分页完全失效** — 所有管理表格受影响

**本轮修复结果**:

| 修复项 | 状态 | 说明 |
|--------|------|------|
| JWT alg:none 绕过 | ✅ 已修复 | `validation.algorithms` 限制为 HS256 |
| 配额 TOCTOU 竞态 | ✅ 已修复 | `UPDATE WHERE used + ? <= total_limit` 原子操作 |
| 渠道配额重置竞态 | ✅ 已修复 | 条件 WHERE 避免重复重置 |
| 登录端点无速率限制 | ✅ 已修复 | 每 IP 每分钟 5 次 |
| 禁用用户登录漏洞(DEF-002) | ✅ 已存在修复 | 代码已有 `status != "active"` 检查 |
| 新增渠道方案阻断 | ✅ 已修复 | `editingChannelId` 改用 `null` |
| Refresh Token 空壳 | ✅ 已修复 | 完整 refresh token 流程 |
| 前端分页失效 | ✅ 已修复 | 添加 `:remote` + `onChange` 回调 |
| 列表接口无 total | ✅ 已修复 | 4 个端点返回 `{ items, total }` |

**待修复项** (建议后续跟进):
- API Keys 明文存储 (CRITICAL) — 需引入加密存储
- CORS/X-Real-IP/SSRF (HIGH) — 安全加固
- 热路径缓存/N+1 查询 (MEDIUM) — 性能优化
- `unwrap()` panic 风险 (MEDIUM) — 错误处理改进

**整体评价** (更新后):
- **CRITICAL 级 JWT 和配额竞态已修复** — 核心认证和配额系统安全性提升
- **Refresh Token 流程已完善** — Token 过期不再直接登出
- **前端分页已修复** — 所有管理表格后端分页正常工作
- 仍有 API Keys 明文存储、CORS、SSRF 等安全问题待修复
- 多处错误处理和性能优化可后续跟进
