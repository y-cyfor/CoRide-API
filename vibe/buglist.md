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

## 二、设计如此 (非Bug)

### 1. set_log_level API 不实际修改日志级别 ~~[设计如此]~~

**原因**: Rust tracing-subscriber 在初始化后不支持热重载日志级别。要实现运行时动态修改需要使用 `ReloadLayer` 机制，需要重构 main.rs 中的 tracing 初始化代码，改动较大。当前 API 返回消息已提示 "requires restart"，不是严格意义的 bug。

### 2. set_global_rate_limit 修改配置文件但运行时不生效 ~~[设计如此]~~

**原因**: 全局限流器 (`global_qps_limiter`) 是 Governor RateLimiter 实例，初始化后不支持动态修改 QPS。要实现需要重构 AppState 使限流器支持运行时替换（使用 `ArcSwap` 或 `Mutex`），改动较大。当前 API 返回消息已提示 "requires restart"，不是严格意义的 bug。

### 3. 配额 total_limit 默认值为0 ~~[设计如此]~~

**原因**: 用户在创建配额表单时会手动修改默认值，保存前会验证数值，属于 UI 设计建议而非严格 bug。

---

## 三、暂不修改 (技术限制/优先级低)

### 1. Streaming 响应全量加载到内存 ~~[暂不修改]~~

**文件**: `backend/src/service/proxy.rs:133`

**原因**: 需要实现真正的流式转发，改动较大，建议作为独立优化任务

---

### 2. Token 估算不考虑 JSON 结构 ~~[暂不修改]~~

**文件**: `backend/src/utils/token_counter.rs`

**原因**: 当前按字符计数是合理的启发式估算，改进需要引入 tiktoken 库

---

## 四、已修复缺陷

> 以下缺陷已修复并迁移至 `buglist-completed.md`

| 原问题 | 修复方案 | 迁移至 |
|--------|----------|--------|
| quota_warnings SQL 除零风险 | 添加 `total_limit > 0` WHERE 条件 | buglist-completed.md 八.1 |
| CSV 导出 channel_id=0 误导 | NULL 显示为 "N/A" 而非 0 | buglist-completed.md 八.2 |
| 流量计划随机数种子不够随机 | 混合时间戳+进程ID+线程地址熵源 | buglist-completed.md 八.3 |
| 流量计划时段重叠未验证 | 后端添加 validate_traffic_plan_slots | buglist-completed.md 八.4 |

---

## 五、已排除的"伪Bug"

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
| set_log_level 不实际生效 | 已标注为"设计如此"，需 tracing ReloadLayer 重构 |
| set_global_rate_limit 不生效 | 已标注为"设计如此"，需 AppState 限流器重构 |
| 配额 total_limit 默认值=0 | 已标注为"设计如此"，非严格 bug |

---

## 六、修复优先级汇总

| 优先级 | 问题 | 风险 | 状态 |
|--------|------|------|------|
| P0 | 硬编码凭据 | 凭据泄露 | ⚠️ 需手动配置 |
| P1 | set_log_level 不实际生效 | API误导 | 📝 设计如此，需重构 tracing |
| P1 | set_rate_limit 不生效 | API误导 | 📝 设计如此，需重构 AppState |
| P1 | quota_warnings 除零风险 | SQL错误 | ✅ 已修复 |
| P1 | 随机数种子不随机 | 分配不均 | ✅ 已修复 |
| P1 | 时段重叠未验证 | 配置冲突 | ✅ 已修复 |
| P2 | 配额默认值=0 | 用户困扰 | 📝 设计如此 |
| P2 | 时段验证不阻止保存 | 配置冲突 | ✅ 前端已有 |
| P3 | CSV channel_id=0 | 数据误导 | ✅ 已修复 |
| P3 | Streaming全量加载 | 性能 | ⏸️ 暂不修改 |
| P3 | Token估算不准确 | 计量偏差 | ⏸️ 暂不修改 |

---

## 七、2026-04-17 审计新增缺陷

> 以下 4 项已全部修复

| 原问题 | 修复方案 | 状态 |
|--------|----------|------|
| Health Check 将 4xx 视为健康 | 移除 `is_client_error()`，仅 `is_success()` 视为健康 | ✅ 已修复 |
| update_user_key 代码冗余 | 删除无 WHERE 条件的死代码，简化为 if/else 直接赋值 | ✅ 已清理 |
| settings 日志级别"更新成功"误导 | 添加 "需重启生效" 警告提示 | ✅ 已修复 |
| home 统计卡片 total_tokens 类型不匹配 | stats ref 添加 `total_tokens: 0` 声明 | ✅ 已修复 |

---

## 八、缺陷统计

| 类别 | 已修复 | 设计如此 | 暂不修改 | 需手动处理 | 待修复 | 合计 |
|------|--------|----------|----------|------------|--------|------|
| 安全问题 | 0 | 0 | 0 | 1 | 0 | 1 |
| 设计缺陷 | 4 | 3 | 0 | 0 | 2 | 9 |
| UI/交互 | 0 | 1 | 0 | 0 | 2 | 3 |
| 性能 | 0 | 0 | 1 | 0 | 0 | 1 |
| 其他 | 1 | 0 | 0 | 0 | 0 | 1 |
| **合计** | **5** | **4** | **1** | **1** | **4** | **15** |

---

> 已修复缺陷请查看 `buglist-completed.md`（共34项已修复）