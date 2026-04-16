# CoRide-API 已完成功能

> 从需求池中已完成的项目归档。
> 最后更新: 2026-04-16

| 编号 | 功能 | 完成说明 |
|------|------|----------|
| 1 | Streaming 流式透传 | `proxy_request_stream` 使用 `bytes_stream` 实现 SSE 透传 |
| 2 | 请求/响应体日志记录 | `log_request_body` / `log_response_body` 配置在 proxy 中生效并写入数据库 |
| 3 | 渠道配额周期自动重置 | `reset_channel_quota_if_expired` 按 hour/day/week/month 周期自动归零 |
| 4 | 用户表单模型绑定 | MultiSelect 组件，从已有模型列表中选择，存为 JSON 数组 |
| 5 | 用户列表 API Key 展示 | 前8位掩码 + 一键复制 |
| 6 | 用户列表配额使用进度 | quota_usage 列，彩色进度条 |
| 7 | 渠道列表配额进度 | 进度条 + 周期文本（如 "150/1000 次/月"） |
| 8 | 仪表盘统计图表 | 近7天趋势折线图 + 渠道饼图 + Token 堆叠柱状图 |
| 9 | 统计页面筛选 | 按渠道/模型/天数筛选 |
| 10 | 日志详情抽屉 | Drawer 组件展示请求体/响应体 JSON + Token 明细 |
| 11 | P95 延迟 + 错误率 | `dashboard_stats` 返回 `p95_latency_ms` 和 `error_rate` |
| 12 | 实时请求表格 | 仪表盘展示最新 10 条请求 |
| 13 | Token 堆叠柱状图 | 输入/输出 Token 分开堆叠展示 |
| 14 | 导出 CSV | `export_logs_csv` 导出请求日志 |
| 15 | 系统设置页面 | 日志级别动态调整 + 全局限流配置保存 |
| 16 | 应用伪装预设管理 | 前端 view + 后端 CRUD，系统内置不可删 |
| 17 | 用户自助配额查询 | `GET /v1/user/info` 返回配额/可用模型/API Key |
| 18 | 渠道健康检查 | 后台每5分钟检测，连续3次失败自动禁用，恢复后自动启用 |
| 19 | 配额预警通知 | `quota_warnings` 接口 + 仪表盘 Alert 卡片（80%/90%/100%） |
| 20 | 供应商预设级联选择 | NCascader 选择 供应商 → 版本 → 自动填充 URL |
| 21 | Traffic Plan 管理 | 全局/渠道级流量计划 CRUD + 前端页面 |
| 22 | JWT 认证 | 真实 JWT 签发/验证 + admin_auth 中间件保护管理接口 |
| 23 | 日志定时清理 | 每小时后台任务，按 retention_days 清理过期日志 |
| 24 | 渠道连通性测试 | 发送真实 `/models` 请求，返回状态码 + 友好提示 |
