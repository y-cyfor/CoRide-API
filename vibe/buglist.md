# CoRide-API Bug 审计报告 - 待修复缺陷

> 审计时间: 2025-04-15 / 2026-04-16
> 已修复缺陷已迁移至 `buglist-completed.md`
> 此文件仅保留待处理问题

---

## 一、待手动处理 (安全相关)

### 1. 硬编码敏感信息 ~~[需手动处理]~~

**文件**: `backend/config/config.yaml:12-16`

**状态**: 配置已支持环境变量覆盖 (`LP_ADMIN_PASSWORD`, `LP_JWT_SECRET`)，需在部署时配置

**风险**: 默认密码 `admin123` 和 JWT secret `change-me-to-random-string` 可能被攻击者利用

**建议**: 在生产环境部署前，务必通过环境变量或修改配置文件设置安全的凭据

---

## 二、暂不修改 (技术限制/优先级低)

### 1. 密码登录无强度校验 ~~[暂不修改]~~

**文件**: `web/src/views/_builtin/login/modules/pwd-login.vue`

**原因**: 密码强度验证应在后端登录和创建用户接口实现，需要修改认证流程

---

### 2. Streaming 响应全量加载到内存 ~~[暂不修改]~~

**文件**: `backend/src/service/proxy.rs:133`

**原因**: 需要实现真正的流式转发，改动较大，建议作为独立优化任务

---

### 3. Token 估算不考虑 JSON 结构 ~~[暂不修改]~~

**文件**: `backend/src/utils/token_counter.rs`

**原因**: 当前按字符计数是合理的启发式估算，改进需要引入 tiktoken 库

---

### 4. 前端无错误边界处理 ~~[暂不修改]~~

**原因**: 需要了解前端架构中的错误处理机制，建议作为独立任务

---

### 5. 无审计日志 ~~[暂不修改]~~

**原因**: 需要新增审计日志表和中间件，建议作为独立功能需求

---

## 三、2026-04-16 新发现待修复缺陷

> 本次审计覆盖：后端 admin_routes.rs、proxy_routes.rs、quota.rs、health.rs、models.rs、proxy.rs + 前端 quota/index.vue、stats/index.vue、model/index.vue、channel/index.vue、traffic-plan/index.vue + 配置文件

---

### P1 设计缺陷/安全问题 (8个)

#### 1. test_channel 每次创建新 HTTP 客户端 ~~[待修复]~~

**问题描述：**
- `admin_routes.rs:391-397` 中 `test_channel` 函数每次测试渠道都新建 `reqwest::Client::builder().timeout(10s).build()`
- 应复用 `AppState` 中已有的共享 HTTP 客户端，避免资源浪费

**涉及文件：** `backend/src/router/admin_routes.rs:391-397`

---

#### 2. update_quota 硬编码 enabled=true ~~[待修复]~~

**问题描述：**
- `admin_routes.rs:848` 中 `let enabled = true;` 硬编码启用状态
- 更新配额时强制启用，忽略用户可能想禁用配额的需求
- `UpdateQuotaRequest` 应包含 `enabled: Option<bool>` 字段

**涉及文件：** `backend/src/router/admin_routes.rs:829-854`

---

#### 3. set_log_level API 不实际修改日志级别 ~~[待修复]~~

**问题描述：**
- `admin_routes.rs:1158-1174` 中 `set_log_level` 只打印日志 `tracing::info!("Log level change requested")`
- 实际上并未动态修改 `tracing-subscriber` 的日志级别
- 返回消息说"需要重启"，但这与 API 名称"设置日志级别"不符

**涉及文件：** `backend/src/router/admin_routes.rs:1158-1174`

**修复方案：** 使用 `tracing_subscriber::reload::ReloadHandle` 实现动态修改

---

#### 4. set_global_rate_limit 修改配置文件但运行时不生效 ~~[待修复]~~

**问题描述：**
- `admin_routes.rs:1196-1227` 通过字符串替换修改 `config/config.yaml`
- 修改后需要重启才能生效，但 API 返回"success"给用户误导
- 运行时的 `AppState.config` 仍是旧值

**涉及文件：** `backend/src/router/admin_routes.rs:1196-1227`

**修复方案：** 添加 `AppState.config` 的动态更新机制，或明确返回"需要重启"

---

#### 5. quota_warnings SQL 除零风险 ~~[待修复]~~

**问题描述：**
- `admin_routes.rs:1241-1243` 查询 `CAST(q.used AS FLOAT) / CAST(q.total_limit AS FLOAT)`
- 当 `total_limit=0` 时会产生除零错误（SQLite 可能返回 NULL 或报错）
- 应先过滤 `total_limit > 0` 的记录

**涉及文件：** `backend/src/router/admin_routes.rs:1229-1267`

---

#### 6. 流量计划随机数种子不够随机 ~~[待修复]~~

**问题描述：**
- `models.rs:1247-1252` 使用 `SystemTime::now().subsec_nanos()` 作为随机种子
- 高并发请求在同一纳秒内可能产生相同种子，导致相同的"随机"选择
- 应使用真正的随机数生成器（如 `rand::rngs::ThreadRng`）

**涉及文件：** `backend/src/db/models.rs:1245-1260`

---

#### 7. API Key 前缀不一致 ~~[待修复]~~

**问题描述：**
- 创建用户时 API Key 前缀为 `sk-` (`admin_routes.rs:290`)
- 重置用户 Key 时前缀为 `lp-` (`admin_routes.rs:307`)
- 前缀不一致可能导致用户困惑，且与品牌名称不匹配

**涉及文件：** `backend/src/router/admin_routes.rs:290, 307`

---

#### 8. 流量计划时段重叠未验证 ~~[待修复]~~

**问题描述：**
- 同一渠道的多个流量计划时段可能重叠（如 0-8 和 6-12）
- 前端 `traffic-plan/index.vue:64-66` 有 `hasOverlap` 检查但未阻止保存
- 后端 `upsert_channel_traffic_plan` 和 `upsert_global_traffic_plan` 未做时段重叠校验

**涉及文件：** 
- `backend/src/router/admin_routes.rs:1302-1335, 1384-1425`
- `web/src/views/routing/traffic-plan/index.vue`

---

### P2 UI/交互缺陷 (6个)

#### 1. 配额 total_limit 默认值为0 ~~[待修复]~~

**问题描述：**
- `quota/index.vue:24` 中 `total_limit: 0` 作为默认值
- 用户可能无意创建零配额，导致用户被拒绝服务
- 应设置合理的默认值（如 10000 tokens）或强制用户输入

**涉及文件：** `web/src/views/control/quota/index.vue:21-25`

---

#### 2. quota loadUsers 未处理分页格式 ~~[待修复]~~

**问题描述：**
- `quota/index.vue:73-78` 中 `userOptions.value = data.map(u => ...)` 
- 当后端返回 `{ items, total }` 格式时，直接对 `data` 对象 `.map()` 会失败
- 应改为 `(data.items || data).map()`

**涉及文件：** `web/src/views/control/quota/index.vue:73-78`

---

#### 3. model loadData 未处理分页格式 ~~[待修复]~~

**问题描述：**
- `model/index.vue:146-150` 中 `models.value = data` 
- 当后端返回 `{ items, total }` 格式时，赋值不正确
- 应改为 `models.value = data.items || data`

**涉及文件：** `web/src/views/upstream/model/index.vue:146-150`

---

#### 4. 统计页面 total_tokens 取值不一致 ~~[待修复]~~

**问题描述：**
- `stats/index.vue:65` 中 `total_tokens: usageRes.data?.total_tokens || 0`
- `dashRes` 和 `usageRes` 来自两个不同的 API，数据可能不一致
- `dashboard_stats` API 不返回 `total_tokens`，依赖 `usage_stats` 补充

**涉及文件：** `web/src/views/data/stats/index.vue:60-67`

---

#### 5. 渠道表单缺少配额默认值 ~~[待修复]~~

**问题描述：**
- `channel/index.vue:21-29` 中 `formModel` 只有基本字段
- 新建渠道时 `quota_type`, `quota_limit`, `quota_cycle` 无默认值
- 编辑渠道时可能丢失已配置的配额信息

**涉及文件：** `web/src/views/upstream/channel/index.vue:21-29`

---

#### 6. 流量计划编辑时段验证未阻止保存 ~~[待修复]~~

**问题描述：**
- `traffic-plan/index.vue:64-66` 的 `hasOverlap` 函数只是检查
- 未在保存时阻止重叠时段的提交
- 用户可能保存冲突的时段配置

**涉及文件：** `web/src/views/routing/traffic-plan/index.vue`

---

### P3 其他缺陷 (5个)

#### 1. usage_stats 动态 SQL 过于复杂 ~~[待优化]~~

**问题描述：**
- `admin_routes.rs:921-1081` 多分支动态拼接 SQL 字符串
- 虽使用参数化查询防止注入，但代码可读性差、难以维护
- 建议使用查询构建器或拆分为多个独立函数

**涉及文件：** `backend/src/router/admin_routes.rs:921-1081`

---

#### 2. export_logs_csv 固定导出10000条 ~~[待优化]~~

**问题描述：**
- `admin_routes.rs:1093` 固定 `LIMIT 10000`
- 不支持按筛选条件导出（如时间范围、渠道、模型）
- 大量日志时可能遗漏重要数据

**涉及文件：** `backend/src/router/admin_routes.rs:1085-1116`

---

#### 3. CSV 导出 channel_id=0 误导 ~~[待修复]~~

**问题描述：**
- `admin_routes.rs:1103` 中 `ch.unwrap_or(0)` 将 NULL 变为 0
- 用户可能误以为日志来自 ID=0 的渠道
- 应导出为空字符串或 "N/A"

**涉及文件：** `backend/src/router/admin_routes.rs:1103`

---

#### 4. monthly 配额周期=30天 ~~[待确认]~~

**问题描述：**
- `quota.rs:111` 中 `monthly` 周期计算为 `now + 30 days`
- 非标准月份天数（28-31天），可能导致月初/月末偏差
- 建议使用 `chrono::Months` 或明确说明"30天周期"

**涉及文件：** `backend/src/service/quota.rs:107-113`

---

#### 5. 渠道配额周期未初始化 ~~[待修复]~~

**问题描述：**
- 创建渠道时 `create_channel` 未设置 `quota_period_start/quota_period_end`
- 只有在 `reset_channel_quota_if_expired` 才会设置
- 首次请求时渠道配额状态可能不正确

**涉及文件：** 
- `backend/src/router/admin_routes.rs:350-366`
- `backend/src/service/quota.rs:79-129`

---

## 四、修复优先级汇总

| 优先级 | 问题 | 风险 | 状态 |
|--------|------|------|------|
| P0 | 硬编码凭据 | 凭据泄露 | ⚠️ 需手动配置 |
| P1 | test_channel 创建新客户端 | 资源浪费 | ⏳ 待修复 |
| P1 | update_quota 硬编码 enabled | 功能缺失 | ⏳ 待修复 |
| P1 | set_log_level 不生效 | API误导 | ⏳ 待修复 |
| P1 | set_rate_limit 不生效 | API误导 | ⏳ 待修复 |
| P1 | quota_warnings 除零风险 | SQL错误 | ⏳ 待修复 |
| P1 | 随机数种子不随机 | 分配不均 | ⏳ 待修复 |
| P1 | API Key前缀不一致 | 用户困惑 | ⏳ 待修复 |
| P1 | 时段重叠未验证 | 配置冲突 | ⏳ 待修复 |
| P2 | 配额默认值=0 | 用户困扰 | ⏳ 待修复 |
| P2 | quota loadUsers 格式 | 功能错误 | ⏳ 待修复 |
| P2 | model loadData 格式 | 功能错误 | ⏳ 待修复 |
| P2 | stats total_tokens | 数据不一致 | ⏳ 待修复 |
| P2 | 渠道表单缺配额字段 | 编辑丢失 | ⏳ 待修复 |
| P2 | 时段验证不阻止保存 | 配置冲突 | ⏳ 待修复 |
| P3 | usage_stats SQL复杂 | 可维护性 | ⏸️ 待优化 |
| P3 | export_csv 固定10000条 | 功能限制 | ⏸️ 待优化 |
| P3 | CSV channel_id=0 | 数据误导 | ⏳ 待修复 |
| P3 | monthly=30天 | 逻辑偏差 | ⏸️ 待确认 |
| P3 | 渠道配额未初始化 | 状态错误 | ⏳ 待修复 |
| P3 | 密码无强度校验 | 安全风险 | ⏸️ 暂不修改 |
| P3 | Streaming全量加载 | 性能 | ⏸️ 暂不修改 |
| P3 | Token估算不准确 | 计量偏差 | ⏸️ 暂不修改 |
| P3 | 无错误边界 | 用户体验 | ⏸️ 暂不修改 |
| P3 | 无审计日志 | 安全审计 | ⏸️ 暂不修改 |

---

## 五、待修复统计

| 类别 | 待修复 | 暂不修改 | 需手动处理 | 合计 |
|------|--------|----------|------------|------|
| 安全问题 | 0 | 0 | 1 | 1 |
| 设计缺陷 | 8 | 0 | 0 | 8 |
| UI/交互 | 6 | 1 | 0 | 7 |
| 性能 | 0 | 1 | 0 | 1 |
| 逻辑Bug | 0 | 1 | 0 | 1 |
| 缺失功能 | 0 | 2 | 0 | 2 |
| 其他 | 3 | 2 | 0 | 5 |
| **合计** | **17** | **6** | **1** | **24** |

---

> 已修复缺陷请查看 `buglist-completed.md`（共30项已修复）