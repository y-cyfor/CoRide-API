# CoRide-API 需求池 (DP)

> 基于 PRD.md 需求文档与实际代码审计，对比当前实现，整理剩余功能缺陷和新增建议。
> 已完成项归档至 `DP-completed.md`。
> 最后更新: 2026-04-16

---

## 一、PRD 已规划但仍有缺陷的功能

### 1. 日志日期范围筛选未真正生效
- **位置**：`web/src/views/data/log/index.vue:160-166`
- **现状**：前端有 `NDatePicker type="daterange"` 控件，但 `filterDateRange` 未传递给后端查询参数
- **PRD要求**：`GET /admin/logs` 支持时间范围筛选
- **修复**：将 `filterDateRange` 转换为 `start_time` / `end_time` 传给 `fetchLogList`

### 2. 渠道列表缺少模型数和应用伪装列
- **位置**：`web/src/views/upstream/channel/index.vue:94-156`
- **PRD要求**：渠道列表应显示「模型数」和「应用伪装」列
- **现状**：表格列仅有 ID/名称/类型/URL/权重/状态/健康/配额/操作
- **优化**：增加两列：
  - 模型数：查询该渠道绑定的模型数量
  - 应用伪装：显示绑定的 app_profile 名称标签

### 3. 供应商预设缺少 Anthropic 兼容 URL
- **位置**：`web/src/views/upstream/channel/index.vue:44-91`
- **PRD要求**：每个供应商提供 OpenAI 兼容 + Anthropic 兼容两套 Base URL
- **现状**：级联选择器仅提供 OpenAI 兼容 URL，没有 Anthropic 兼容选项
- **优化**：在每个供应商下增加接口类型分支：
  ```
  阿里云 → 标准版 → OpenAI 兼容 (url + type=openai)
  阿里云 → 标准版 → Anthropic 兼容 (url + type=anthropic)
  阿里云 → CodingPlan → OpenAI 兼容
  阿里云 → CodingPlan → Anthropic 兼容
  ```

### 4. 配额列表缺少用户信息展示
- **位置**：`web/src/views/control/quota/index.vue:42-61`
- **现状**：配额列表仅显示 `user_id`（数字），不显示用户名
- **PRD要求**：配额列表显示「用户」列（用户名）
- **优化**：在渲染时关联用户名，或后端返回时 JOIN users 表附带 username

### 5. 统计页面缺少调用排名和模型分布图表
- **位置**：`web/src/views/data/stats/index.vue`
- **PRD要求**：
  - 用户调用排名柱状图（TOP 10）
  - 模型调用分布饼图
- **现状**：仅有趋势折线图、渠道饼图、Token 柱状图，缺少用户排名和模型分布
- **优化**：增加两个图表，后端 `usage_stats` 已返回 `top_users` 数据，前端缺少渲染

### 6. 统计页面缺少成功/失败次数和 P95 耗时卡片
- **位置**：`web/src/views/data/stats/index.vue:230-252`
- **PRD要求**：统计卡片包含总调用次数、成功次数、失败次数、总 Token、平均耗时、P95 耗时、错误率
- **现状**：仅有总请求数、今日请求、活跃用户、总 Token 四个卡片
- **优化**：后端 `dashboard_stats` 已返回 `p95_latency_ms` 和 `error_rate`，前端未展示

### 7. 前端日志列表缺少用户名显示
- **位置**：`web/src/views/data/log/index.vue:35`
- **现状**：显示原始 `user_api_key`，未关联显示用户名
- **PRD要求**：日志列表「用户」列显示用户名
- **优化**：后端 list_logs 时 JOIN users 表，返回 username；或前端根据 API Key 反查

### 8. 限流配置页面缺少更新功能
- **位置**：`web/src/views/control/ratelimit/index.vue`
- **现状**：仅有创建和删除操作，缺少编辑已有规则的入口
- **PRD要求**：`PUT /admin/ratelimits/:id` 支持更新
- **优化**：增加编辑按钮和编辑表单

### 9. 渠道测试缺少耗时展示
- **位置**：`backend/src/router/admin_routes.rs:379-465`
- **PRD要求**：返回成功/失败**及耗时**
- **现状**：`test_channel` 返回 `status`、`http_status`、`message`，但没有测量和返回延迟时间
- **优化**：记录请求开始/结束时间，返回 `latency_ms` 字段

### 10. 系统设置页面缺少代理配置修改能力
- **位置**：`web/src/views/settings/index.vue:139-159`
- **现状**：代理配置（超时、重试、记录请求/响应体）为只读展示
- **PRD要求**：系统设置支持服务器配置动态调整
- **优化**：增加代理配置的编辑和保存功能（需后端新增接口）

---

## 二、新增功能建议

### 11. 配置导入导出
- **新增**：导出/导入渠道、模型、用户、配额等配置
- **格式**：JSON 文件
- **用途**：备份恢复、多实例同步

### 12. 批量操作
- **新增**：
  - 渠道列表：批量启用/禁用
  - 日志列表：按时间范围批量删除
  - 配额列表：批量重置已使用量

### 13. 模型管理增强
- **新增**：创建渠道时，一键导入该供应商的常见模型
- **示例**：选择阿里云渠道时，自动添加 qwen-turbo / qwen-plus / qwen-max 等

### 14. 渠道 API Key 轮询可视化
- **现状**：多 Key 轮询逻辑存在，但管理员无法看到轮询状态
- **新增**：渠道详情显示 API Key 列表及当前轮询索引/使用次数

---

## 三、不应纳入的功能

| 编号 | 功能 | 排除原因 |
|------|------|----------|
| X1 | `/v1/embeddings` 等非对话端点 | 项目定位为 LLM 对话 API 中转 |
| X2 | 多租户/多组织隔离 | 目标用户为个人/小团队 |
| X3 | 支付/计费系统 | 非商业化项目 |
| X4 | SSO/OAuth 第三方登录 | PRD 明确"无注册，管理员手动创建" |
| X5 | Redis 集群 | PRD v2.x 规划 |
| X6 | 插件系统 | PRD v2.x 规划 |

---

## 四、执行优先级

| 优先级 | 编号 | 说明 | 工作量 |
|--------|------|------|--------|
| **P0** | 1, 3, 4 | 功能缺陷（日志筛选无效、缺少 Anthropic URL、配额无用户名） | 小 |
| **P1** | 2, 5, 6, 7 | 前端展示不完整（渠道列、用户排名、统计卡片、日志用户名） | 小 |
| **P1** | 9, 10 | 接口功能不完整（测试耗时、代理配置编辑） | 小 |
| **P2** | 8, 13, 14 | 体验优化（限流编辑、模型快速导入、Key 轮询可视化） | 中 |
| **P3** | 11, 12 | 运维功能（导入导出、批量操作） | 中 |
