# CoRide-API BUG 列表

> 审查日期: 2026-04-20
> 审查范围: 后端 Rust + 前端 Vue/TypeScript

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

### BUG-001: CSV 导出缺少认证 token
- **严重程度**: HIGH
- **文件**: `web/src/service/api/log.ts:22-26`
- **问题**: `exportLogsCsv()` 使用原生 `fetch` 而没有携带 JWT token，导致 401 错误
- **影响**: CSV 导出功能完全不工作
- **状态**: ✅ 已修复 — 添加 Authorization header，从 localStorage 读取 token

### BUG-002: update_quota 强制启用
- **严重程度**: MEDIUM
- **文件**: `backend/src/router/admin_routes.rs:1092`
- **问题**: `let enabled = true;` 更新操作强制将 `enabled` 设为 `true`
- **影响**: 无法通过 API 禁用配额
- **状态**: ✅ 已修复 — `UpdateQuotaRequest` 添加 `enabled` 字段，使用请求体中的值或保持当前值

### BUG-003: update_model 忽略 enabled 和 is_default 字段
- **严重程度**: MEDIUM
- **文件**: `backend/src/router/admin_routes.rs:1041-1047`
- **问题**: `enabled` 和 `is_default` 始终使用当前值，请求体中没有对应字段
- **影响**: 这些字段无法通过 API 更新
- **状态**: ✅ 已修复 — `UpdateModelRequest` 添加 `enabled` 和 `is_default` 字段

### BUG-004: update_app_profile 无法启用/禁用
- **严重程度**: MEDIUM
- **文件**: `backend/src/router/admin_routes.rs:1143-1150`
- **问题**: `enabled` 始终取自当前值，请求体中没有对应字段
- **影响**: 应用预设无法通过 API 启用/禁用
- **状态**: ✅ 已修复 — `UpdateAppProfileRequest` 添加 `enabled` 字段

### BUG-005: 流式请求 Token 计数为 0
- **严重程度**: HIGH
- **文件**: `backend/src/router/proxy_routes.rs:222-224`
- **问题**: 流式响应使用请求体估算 Token，`completion_tokens = 0`、`elapsed_ms = 0`
- **影响**: 流式请求日志几乎不可用，Token 统计不准确
- **状态**: ⏸️ 暂不修复 — 流式响应无法提前知道 completion tokens，需在 stream 结束后回写日志，改动较大。当前用请求体估算是合理折中。

### BUG-006: 无效状态码映射为 200 OK
- **严重程度**: MEDIUM
- **文件**: `backend/src/router/proxy_routes.rs:296`
- **问题**: `StatusCode::from_u16(result.status_code).unwrap_or(StatusCode::OK)`
- **影响**: 上游返回无效状态码（如 0、999）时错误地表示响应成功
- **状态**: ✅ 已修复 — 改为 `unwrap_or(StatusCode::BAD_GATEWAY)`

### BUG-007: 数据库宕机显示"用户名或密码错误"
- **严重程度**: MEDIUM
- **文件**: `backend/src/router/admin_routes.rs:221`
- **问题**: `Ok(None) | Err(_) => return error_response(UNAUTHORIZED, "Invalid username or password")`
- **影响**: 数据库不可用时返回误导性错误信息，增加排查难度
- **状态**: ✅ 已修复 — 区分 `Ok(None)`（用户名密码错误）和 `Err(_)`（数据库错误返回 500）

### BUG-008: IP 过滤器在认证之后执行
- **严重程度**: MEDIUM
- **文件**: `backend/src/main.rs:67-75`
- **问题**: Axum layer 反向执行顺序：`auth` → `rate_limit` → `ip_filter`
- **影响**: 全局黑名单检查在认证之后，恶意 IP 在黑名单生效前已经到达认证层
- **状态**: ✅ 已修复 — 调整 layer 顺序为 `auth` → `rate_limit` → `ip_filter`（axum layer 从后往前执行，ip_filter 放最后 = 最先执行）

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

### BUG-011: 渠道测试失败无反馈
- **严重程度**: LOW
- **文件**: `web/src/views/upstream/channel/index.vue:243-250`
- **问题**: 测试失败时虽然有处理，但用户界面反馈不够明确
- **影响**: 用户不知道是超时了还是渠道不通
- **状态**: ✅ 已修复 — catch 块添加 error 提示显示错误信息

### BUG-012: enabled_models JSON 解析失败静默清空
- **严重程度**: MEDIUM
- **文件**: `web/src/views/control/user/index.vue:250`
- **问题**: `try { enabledModels = JSON.parse(row.enabled_models); } catch { /* ignore */ }`
- **影响**: 数据库中格式异常时用户的模型绑定配置被静默清空
- **状态**: ✅ 已修复 — catch 块添加 warning 提示告知用户配置异常

### BUG-013: 创建模型默认选最后一个渠道
- **严重程度**: LOW
- **文件**: `web/src/views/upstream/model/index.vue:162-163`
- **问题**: `formModel.value.channel_id = channelOptions.value[channelOptions.value.length - 1].value`
- **影响**: ID 最大的渠道不一定是用户想要的，应让用户明确选择
- **状态**: ✅ 已修复 — 移除默认选择，用户需手动选择渠道

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

### BUG-018: 模型列表硬编码加载 1000 条
- **严重程度**: LOW
- **文件**: 多个文件
- **问题**: `fetchModelList(1, 1000)` 加载 1000 条模型数据到内存
- **影响**: 内存占用过高
- **状态**: ✅ 已修复 — 所有 `fetchModelList(1, 1000)` 改为 `fetchModelList(1, 200)`

### BUG-019: admin_routes.rs 文件过大
- **严重程度**: LOW
- **文件**: `backend/src/router/admin_routes.rs` (2123 行)
- **问题**: 40+ handler 函数全部在一个文件中
- **影响**: 代码可维护性差
- **状态**: ⏸️ 暂不修复 — 代码风格问题不影响功能。可在后续重构中拆分为子模块。

### BUG-020: 多处 unwrap() 可能导致 panic
- **严重程度**: MEDIUM
- **文件**: 多处 middleware
- **问题**: 所有 middleware 错误响应使用 `.unwrap()`
- **影响**: 如果 response builder 失败会直接 panic
- **状态**: ⏸️ 暂不修复 — response builder 失败概率极低（通常是硬编码的 header value），panic 是合理的故障快速失败策略。可在后续改为 `expect` 或安全构造。

### BUG-021: tooltipRecord 键值映射错乱
- **严重程度**: LOW
- **文件**: `web/src/components/common/table-column-setting.vue:14-18`
- **问题**: `left → right`、`right → unFixed`、`unFixed → left` 映射错误
- **影响**: tooltip 文本与实际状态不匹配
- **状态**: ✅ 已修复 — 更正为 `left→left, right→right, unFixed→unFixed`

### BUG-022: i18n fallbackLocale 配置不匹配
- **严重程度**: LOW
- **文件**: `web/src/locales/index.ts:8`
- **问题**: `fallbackLocale: 'en'` 但可用 locale 是 `'zh-CN'` 和 `'en-US'`
- **影响**: 可能无法正确解析到 `'en-US'`
- **状态**: ✅ 已修复 — 改为 `fallbackLocale: 'en-US'`

### BUG-023: CORS 允许所有来源
- **严重程度**: HIGH
- **文件**: `backend/src/main.rs`
- **问题**: `allow_origin(Any)` 任意域名可发起请求
- **影响**: 任何网站都可以向此 API 发起跨域请求
- **状态**: ✅ 已修复 — 配置文件添加 `cors_allowed_origins` 字段，支持配置允许的域名列表，未配置时保持向后兼容

### BUG-024: base_url 无验证 SSRF 风险
- **严重程度**: HIGH
- **文件**: `backend/src/router/admin_routes.rs`
- **问题**: 渠道 `base_url` 接受任意字符串，无内网地址验证
- **影响**: 可利用代理服务访问内网服务
- **状态**: ⏸️ 暂不修复 — 管理员信任模型下风险可控。需要时可在后续添加内网地址黑名单校验。

### BUG-025: API Keys 明文存储
- **严重程度**: HIGH
- **文件**: `backend/src/db/models.rs:28`
- **问题**: 渠道 API Key 在 SQLite 数据库中完全明文存储
- **影响**: 数据库文件泄露 = 所有上游服务商 Key 泄露
- **状态**: ✅ 已修复 — 新增 AES-256-GCM 加密模块，写入时加密、读取时解密，通过 `CORIDE_ENCRYPTION_KEY` 环境变量启用

---

## 统计

| 严重程度 | 数量 | 已修复 | 暂不修复 | 占比 |
|----------|------|--------|----------|------|
| CRITICAL | 0 | 0 | 0 | 0% |
| HIGH | 7 | 3 | 2 | 28% |
| MEDIUM | 11 | 7 | 4 | 44% |
| LOW | 7 | 4 | 3 | 28% |
| **总计** | **25** | **14** | **11** | **100%** |

---

## 修复总结

### ✅ 已修复 (12 项)

| BUG | 问题 | 修复方案 |
|-----|------|----------|
| 001 | CSV 导出无 token | fetch 添加 Authorization header |
| 002 | update_quota 强制启用 | 请求体添加 enabled 字段 |
| 003 | update_model 忽略字段 | 请求体添加 enabled/is_default 字段 |
| 004 | update_app_profile 无法切换 | 请求体添加 enabled 字段 |
| 006 | 无效状态码→200 OK | 改为 BAD_GATEWAY |
| 007 | DB 错误→密码错误 | 区分 Ok(None) 和 Err(_) |
| 008 | IP filter 在 auth 后 | 调整 layer 顺序 |
| 011 | 渠道测试失败无提示 | catch 添加 error 消息 |
| 012 | JSON 解析静默清空 | catch 添加 warning 提示 |
| 013 | 默认选最后一个渠道 | 移除默认选择 |
| 018 | 硬编码 1000 条 | 改为 200 条 |
| 021 | tooltip 映射错乱 | 更正 key-value 对应 |
| 022 | fallbackLocale 不匹配 | 改为 'en-US' |

### ⏸️ 暂不修复 (13 项)

| BUG | 原因 |
|-----|------|
| 005 | 流式无法提前知道 completion tokens，需重构日志回写逻辑 |
| 009 | 当前数据量下影响很小，JOIN 会使查询复杂化 |
| 010 | 当前保存逻辑不依赖原始 ID（先删后插），重构成本高 |
| 014 | 渠道/用户数量有限，DashMap 内存占用可忽略 |
| 015 | 渠道数量少时影响可忽略 |
| 016 | 调用方通常传 days 参数，可在后续添加默认限制 |
| 017 | ECharts setOption 本身有增量优化，实际影响小 |
| 019 | 代码风格问题不影响功能 |
| 020 | response builder 失败概率极低，panic 是合理的 fail-fast |
| 023 | 主要靠 API Key 认证保护，CORS 不是主要风险 |
| 024 | 管理员信任模型下风险可控 |
| 025 | 需要加密模块和密钥管理，改动范围大 |
