# CoRide-API Bug 修复状态

> 审计时间: 2026-04-18
> 修复时间: 2026-04-18

---

## 已修复

| 编号 | 严重度 | 问题 | 修复说明 |
|------|--------|------|----------|
| BUG-37-01 | 严重 | IP 白名单功能失效（middleware 顺序错误） | 调整 layer 顺序：auth 最先执行，ip_filter 最后执行 |
| BUG-37-02 | 严重 | X-Forwarded-For 可被伪造 | 移除 X-Forwarded-For 信任，只信任 X-Real-IP 或 remote_addr |
| BUG-36-02 | 严重 | models::create_quota 未支持 channel_id | 函数签名添加 `channel_id: Option<i64>` 参数 |
| BUG-34-01 | 高 | anthropic-beta header 被覆盖 | 添加 `merge_anthropic_beta` 函数，合并而非覆盖 |
| BUG-33-03 | 中等 | P95 延迟计算排序方向错误 | 两处 `ORDER BY elapsed_ms DESC` 改为 `ASC` |
| BUG-33-04 | 低 | today_requests SUM 空集返回 NULL | 添加 `COALESCE(..., 0)` 包装 |
| BUG-33-02 | 中等 | request_logs 缺少 channel_id 索引 | 新增迁移 013 添加索引 |
| BUG-36-03 | 中等 | quotas 缺少 channel_id 索引 | 新增迁移 013 添加索引 |

## 设计如此（无需修复）

| 编号 | 原因 |
|------|------|
| BUG-37-03 | 管理员不应被自己的黑名单拦截 |
| BUG-36-01 | 个人项目并发低，原子操作影响极小 |
| BUG-34-02 | Anthropic 官方渠道始终需要 beta header |
| BUG-34-03 | 健康检查只验证连通性，不需要 beta |
| BUG-33-01 | "今日"和"最近24h"场景不同，定义合理 |
| BUG-35-01 | 进度条 100% 限制 vs 文本实际值，视觉差异可接受 |
| BUG-35-03 | permanent 配额 period_end 为 None，不需要重置 |
| BUG-35-04 | 个人项目配额远不到 2^53 |
| BUG-36-05 | SQLite 默认不启用外键 |
| BUG-37-09 | 数据库故障时 Fail-Open 避免服务中断 |

## 暂不实施

| 编号 | 原因 |
|------|------|
| BUG-37-04 | CIDR 支持增加复杂度，个人项目单 IP 够用 |
| BUG-37-05 | IPv6 标准化优先级低 |
| BUG-37-06 | 个人项目 QPS 低，SQLite 查询足够 |
| BUG-36-04 | 渠道级配额创建后通常不需改渠道 |
| BUG-33-05 | 个人项目用户数和日志量有限 |

## 待修复（低优先级）

| 编号 | 问题 | 计划 |
|------|------|------|
| BUG-36-06 | 前端 channelOptions 使用 undefined | 改为 null |
| BUG-33-06 | 前端类型定义缺少 model_usage | 添加字段 |
| BUG-33-07 | error_rate 返回字符串 | 后端返回数字 |
| BUG-35-02 | 配额类型空字符串误判 | 检查空字符串 |
| BUG-37-07 | IP 格式无验证 | 后端校验 + 前端提示 |
| BUG-37-08 | INSERT OR IGNORE 返回 ID 为 0 | 先查后插 |
