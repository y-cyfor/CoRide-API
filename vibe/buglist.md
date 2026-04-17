# CoRide-API Bug 审计报告 - 待修复缺陷

> 审计时间: 2026-04-17
> 已修复缺陷已迁移至 `buglist-completed.md`
> 此文件仅保留待处理问题

---

## 一、待手动处理 (安全相关)

### 1. 硬编码敏感信息 ~~[需手动处理]~~

**文件**: `backend/config/config.yaml:12-16`

**状态**: 配置已支持环境变量覆盖，需在部署时配置

---

## 二、设计如此 (非Bug)

### 1. set_log_level API 不实际修改日志级别 ~~[设计如此]~~

**原因**: Rust tracing-subscriber 不支持热重载，需要重启

### 2. set_global_rate_limit 修改配置文件但运行时不生效 ~~[设计如此]~~

**原因**: Governor RateLimiter 不支持动态修改，需要重启

### 3. 配额 total_limit 默认值为0 ~~[设计如此]~~

**原因**: 用户会手动修改默认值，属于 UI 设计建议

---

## 三、暂不修改 (技术限制/优先级低)

### 1. Streaming 响应全量加载到内存 ~~[暂不修改]~~

**原因**: 需要实现真正的流式转发，改动较大

### 2. Token 估算不考虑 JSON 结构 ~~[暂不修改]~~

**原因**: 当前按字符计数是合理的启发式估算

---

## 四、已修复缺陷

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
| 用户路由使用 admin 中间件 | 创建 user_auth_middleware | buglist-completed.md 十.1 |
| Key 管理路由权限为 admin | 移除 roles 限制 | buglist-completed.md 十.2 |
| 仪表盘调用 admin API | 根据 isAdmin 调用不同 API | buglist-completed.md 十.3 |
| 数据统计调用 admin API | 根据 isAdmin 调用不同 API | buglist-completed.md 十.4 |
| 后端缺少用户专用 API | 添加 /user/stats/* 和 /user/logs | buglist-completed.md 十.5 |

---

## 五、已排除的"伪Bug"

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

## 六、待处理问题

| 优先级 | 问题 | 风险 | 状态 |
|--------|------|------|------|
| P0 | 硬编码凭据 | 凭据泄露 | ⚠️ 需手动配置 |
| P1 | set_log_level 不实际生效 | 设计如此 | 📝 需重构 tracing |
| P1 | set_rate_limit 不生效 | 设计如此 | 📝 需重构 AppState |
| P2 | 配额默认值=0 | 设计如此 | 📝 UI 建议 |
| P3 | Streaming全量加载 | 暂不修改 | ⏸️ 技术限制 |
| P3 | Token估算不准确 | 暂不修改 | ⏸️ 技术限制 |

---

## 七、缺陷统计

| 类别 | 设计如此 | 暂不修改 | 需手动处理 | 合计 |
|------|----------|----------|------------|------|
| 安全问题 | 0 | 0 | 1 | 1 |
| 设计缺陷 | 2 | 0 | 0 | 2 |
| 性能 | 0 | 1 | 0 | 1 |
| 其他 | 0 | 1 | 0 | 1 |
| **合计** | **2** | **2** | **1** | **5** |

---

> 已修复缺陷请查看 `buglist-completed.md`（共43项已修复）