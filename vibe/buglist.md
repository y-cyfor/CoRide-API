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

### 1. Streaming 响应全量加载到内存 ~~[暂不修改]~~

**文件**: `backend/src/service/proxy.rs:133`

**原因**: 需要实现真正的流式转发，改动较大，建议作为独立优化任务

---

### 2. Token 估算不考虑 JSON 结构 ~~[暂不修改]~~

**文件**: `backend/src/utils/token_counter.rs`

**原因**: 当前按字符计数是合理的启发式估算，改进需要引入 tiktoken 库

---

## 三、2026-04-16 新发现缺陷

> 本次审计覆盖：后端 admin_routes.rs、models.rs、quota.rs + 前端相关组件

---

### P1 真正的 Bug (5个)

#### 1. set_log_level API 不实际修改日志级别 ~~[待修复]~~

**问题描述：**
- `admin_routes.rs:1158-1174` 中 `set_log_level` 只打印日志，未动态修改 tracing 级别
- API 名称和响应都暗示可以"设置"，实际需要重启才能生效

**涉及文件：** `backend/src/router/admin_routes.rs:1158-1174`

---

#### 2. set_global_rate_limit 修改配置文件但运行时不生效 ~~[待修复]~~

**问题描述：**
- `admin_routes.rs:1196-1227` 修改 config.yaml 但 `AppState.config` 仍是旧值
- API 返回 success，误导用户认为已生效

**涉及文件：** `backend/src/router/admin_routes.rs:1196-1227`

---

#### 3. quota_warnings SQL 除零风险 ~~[待修复]~~

**问题描述：**
- `admin_routes.rs:1238` 查询 `CAST(q.used AS FLOAT) / CAST(q.total_limit AS FLOAT)`
- 当 `total_limit=0` 时产生除零错误

**涉及文件：** `backend/src/router/admin_routes.rs:1235-1243`

---

#### 4. 流量计划随机数种子不够随机 ~~[待修复]~~

**问题描述：**
- `models.rs:1247-1252` 使用纳秒时间戳做随机种子
- 高并发时可能产生相同种子，导致不公平分配

**涉及文件：** `backend/src/db/models.rs:1245-1260`

---

#### 5. 流量计划时段重叠未验证 ~~[待修复]~~

**问题描述：**
- 后端 `upsert_global_traffic_plan` 和 `upsert_channel_traffic_plan` 未校验时段重叠
- 用户可配置 0-8 和 6-12 这样的重叠时段

**涉及文件：** `backend/src/router/admin_routes.rs:1302-1335, 1384-1425`

---

### P2 UI/交互缺陷 (2个)

#### 1. 配额 total_limit 默认值为0 ~~[待确认]~~

**问题描述：**
- `quota/index.vue:23` 中 `total_limit: 0` 作为默认值
- 用户可能无意创建零配额

**说明：** 用户保存前可以修改默认值，属于 UI 设计建议而非严格 bug

**涉及文件：** `web/src/views/control/quota/index.vue:21-25`

---

#### 2. 流量计划编辑时段验证未阻止保存 ~~[待修复]~~

**问题描述：**
- `traffic-plan/index.vue:64-66` 的 `hasOverlap` 只是检查函数
- 前端未在保存时调用此函数阻止重叠提交

**涉及文件：** `web/src/views/routing/traffic-plan/index.vue`

---

### P3 其他缺陷 (1个)

#### 1. CSV 导出 channel_id=0 误导 ~~[待修复]~~

**问题描述：**
- `admin_routes.rs:1103` 中 `ch.unwrap_or(0)` 将 NULL 变为 0
- 用户可能误以为日志来自 ID=0 的渠道

**涉及文件：** `backend/src/router/admin_routes.rs:1103`

---

## 四、已排除的"伪Bug"

以下问题经代码审查确认不存在或不是bug：

| 原"问题" | 原因 |
|----------|------|
| test_channel 每次创建新 HTTP 客户端 | 低频管理功能，创建客户端开销可忽略，非 bug |
| update_quota 硬编码 enabled=true | 设计决策（注释：keep enabled on update），非 bug |
| API Key 前缀不一致 | 可能是设计决策（区分创建/重置），需确认 |
| quota loadUsers 未处理分页格式 | 后端 `list_users` 返回数组而非 `{items, total}`，前端正确 |
| model loadData 未处理分页格式 | 后端 `list_models_endpoint` 返回数组而非 `{items, total}`，前端正确 |
| 统计页面 total_tokens 取值不一致 | 两个 API 返回不同数据是设计问题，非 bug |
| 渠道表单缺少配额默认值 | 编辑时从后端加载，非 bug |
| usage_stats 动态 SQL 过于复杂 | 代码可维护性建议，非 bug |
| export_logs_csv 固定10000条 | 功能限制，非 bug |
| monthly 配额周期=30天 | 设计决策，可文档化说明 |

---

## 五、修复优先级汇总

| 优先级 | 问题 | 风险 | 状态 |
|--------|------|------|------|
| P0 | 硬编码凭据 | 凭据泄露 | ⚠️ 需手动配置 |
| P1 | set_log_level 不生效 | API误导 | ⏳ 待修复 |
| P1 | set_rate_limit 不生效 | API误导 | ⏳ 待修复 |
| P1 | quota_warnings 除零风险 | SQL错误 | ⏳ 待修复 |
| P1 | 随机数种子不随机 | 分配不均 | ⏳ 待修复 |
| P1 | 时段重叠未验证 | 配置冲突 | ⏳ 待修复 |
| P2 | 配额默认值=0 | 用户困扰 | ⏸️ 待确认 |
| P2 | 时段验证不阻止保存 | 配置冲突 | ⏳ 待修复 |
| P3 | CSV channel_id=0 | 数据误导 | ⏳ 待修复 |
| P3 | Streaming全量加载 | 性能 | ⏸️ 暂不修改 |
| P3 | Token估算不准确 | 计量偏差 | ⏸️ 暂不修改 |

---

## 六、待修复统计

| 类别 | 待修复 | 待确认 | 暂不修改 | 需手动处理 | 合计 |
|------|--------|--------|----------|------------|------|
| 安全问题 | 0 | 0 | 0 | 1 | 1 |
| 设计缺陷 | 5 | 0 | 0 | 0 | 5 |
| UI/交互 | 1 | 1 | 0 | 0 | 2 |
| 性能 | 0 | 0 | 1 | 0 | 1 |
| 其他 | 1 | 0 | 0 | 0 | 1 |
| **合计** | **7** | **1** | **1** | **1** | **10** |

---

> 已修复缺陷请查看 `buglist-completed.md`（共30项已修复）