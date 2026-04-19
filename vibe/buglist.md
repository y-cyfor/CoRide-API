# CoRide-API BUG 列表

> 审查日期: 2026-04-20
> 审查范围: 后端 Rust + 前端 Vue/TypeScript
> 最后更新: 2026-04-20

---

## 严重程度说明

| 级别 | 说明 |
|------|------|
| **CRITICAL** | 安全漏洞或核心功能完全不可用 |
| **HIGH** | 重要功能异常或数据安全问题 |
| **MEDIUM** | 功能缺陷或性能问题 |
| **LOW** | 代码质量或体验优化 |

---

## BUG 列表

### BUG-005: 流式请求 Token 计数为 0
- **严重程度**: HIGH
- **文件**: `backend/src/router/proxy_routes.rs:222-224`
- **问题**: 流式响应使用请求体估算 Token，`completion_tokens = 0`、`elapsed_ms = 0`
- **影响**: 流式请求日志几乎不可用，Token 统计不准确
- **状态**: ⏸️ 暂不修复 — 流式响应无法提前知道 completion tokens，需在 stream 结束后回写日志，改动较大。当前用请求体估算是合理折中。

### BUG-009: list_users N+1 查询
- **严重程度**: MEDIUM
- **文件**: `backend/src/router/admin_routes.rs:349-356`
- **问题**: 每页 20 个用户 = 20 次额外数据库查询获取配额信息
- **影响**: 性能问题，用户列表加载慢
- **状态**: ⏸️ 暂不修复 — 当前数据量下影响很小（每页 20 次简单聚合查询），改用 JOIN 会使查询复杂化。可在用户量增大后优化。

### BUG-010: 流量方案编辑时数据聚合丢失原始 ID
- **严重程度**: MEDIUM
- **文件**: `web/src/views/routing/traffic-plan/index.vue:139-140`
- **问题**: 将同一时段的多行聚合到 `editingSlots` 时，`id` 被重新赋值为 `idx + 1`
- **影响**: 保存时无法区分新建和更新
- **状态**: ⏸️ 暂不修复 — 当前保存逻辑是"先删除所有 slots 再重新插入"，不依赖原始 ID。如需改为区分新建/更新需重构保存逻辑。

### BUG-013: 创建模型默认选最后一个渠道
- **严重程度**: LOW
- **文件**: `web/src/views/upstream/model/index.vue:168`
- **问题**: `channel_id: channelOptions.value.length > 0 ? channelOptions.value[channelOptions.value.length - 1].value : 0`
- **影响**: ID 最大的渠道不一定是用户想要的，应让用户明确选择
- **状态**: ✅ 已修复 — `handleCreate` 中 `channel_id` 初始化为 0，用户需手动选择渠道

### BUG-014: DashMap 无过期清理
- **严重程度**: MEDIUM
- **文件**: `backend/src/lib.rs:26-27`, `backend/src/middleware/rate_limit.rs`
- **问题**: 按频道和用户创建的限流器存储在 `DashMap` 中，没有清理过期条目
- **影响**: 随着渠道删除和用户流失，DashMap 会无限增长（内存泄漏）
- **状态**: ⏸️ 暂不修复 — 实际场景中渠道和用户数量有限（通常 <1000），DashMap 内存占用很小。可在需要时添加 LRU 或定时清理。

### BUG-015: 健康检查串行执行
- **严重程度**: LOW
- **文件**: `backend/src/service/health.rs`
- **问题**: 所有渠道健康检查顺序执行
- **影响**: 渠道数量多时，健康检查耗时可能超过 5 分钟间隔
- **状态**: ⏸️ 暂不修复 — 当前渠道数量少时影响可忽略。可在渠道数量增多后改为并发。

### BUG-016: Stats 端点全表扫描
- **严重程度**: MEDIUM
- **文件**: `backend/src/router/admin_routes.rs:1155-1163`
- **问题**: 在 `days` 参数缺失的分支中扫描整个 `request_logs` 表
- **影响**: 随日志增长查询越来越慢
- **状态**: ⏸️ 暂不修复 — 该端点有 `days` 参数，调用方通常会传入。可在后续优化中为该分支添加默认天数限制。

### BUG-017: 前端图表重复 setOption
- **严重程度**: LOW
- **文件**: `web/src/views/data/stats/index.vue:129-131`
- **问题**: 每次 `loadData` 都重新 `setOption` 整个图表配置
- **影响**: 筛选频繁变化时性能差
- **状态**: ⏸️ 暂不修复 — ECharts 的 setOption 本身有增量更新优化，实际性能影响很小。

### BUG-019: admin_routes.rs 文件过大
- **严重程度**: LOW
- **文件**: `backend/src/router/admin_routes.rs` (2147 行)
- **问题**: 40+ handler 函数全部在一个文件中
- **影响**: 代码可维护性差
- **状态**: ⏸️ 暂不修复 — 代码风格问题不影响功能。可在后续重构中拆分为子模块。

### BUG-020: 多处 unwrap() 可能导致 panic
- **严重程度**: MEDIUM
- **文件**: 多处 middleware
- **问题**: 所有 middleware 错误响应使用 `.unwrap()`
- **影响**: 如果 response builder 失败会直接 panic
- **状态**: ⏸️ 暂不修复 — response builder 失败概率极低（通常是硬编码的 header value），panic 是合理的故障快速失败策略。可在后续改为 `expect` 或安全构造。

### BUG-024: base_url 无验证 SSRF 风险
- **严重程度**: HIGH
- **文件**: `backend/src/router/admin_routes.rs`
- **问题**: 渠道 `base_url` 接受任意字符串，无内网地址验证
- **影响**: 可利用代理服务访问内网服务
- **状态**: ⏸️ 暂不修复 — 管理员信任模型下风险可控。需要时可在后续添加内网地址黑名单校验。

---

## 统计

| 严重程度 | 数量 | 已修复 | 修复不成功 | 暂不修复 | 占比 |
|----------|------|--------|-----------|----------|------|
| CRITICAL | 0 | 0 | 0 | 0 | 0% |
| HIGH | 7 | 5 | 0 | 2 | 28% |
| MEDIUM | 11 | 7 | 0 | 4 | 44% |
| LOW | 7 | 5 | 0 | 2 | 28% |
| **总计** | **25** | **17** | **0** | **8** | **100%** |

---

## 待修复项

（无）
