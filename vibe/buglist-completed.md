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
| **合计** | **38** |

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

**涉及文件：** `web/src/views/home/index.vue:17