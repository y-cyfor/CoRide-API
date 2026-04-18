# 渠道用量统计面板 + 配额可视化 设计

> 日期: 2026-04-18
> 对应需求池: #20 渠道用量统计面板, #22 渠道配额可视化

## 概述

在渠道管理页面（`upstream/channel/index.vue`）每行渠道上叠加显示用量统计和配额进度条。纯统计展示，不依赖上游 API，不新增数据库表。

## 后端改动

### 1. 新增接口 `GET /admin/channels/{id}/stats`

返回单个渠道的详细用量统计：

```json
{
  "total_requests": 12345,
  "today_requests": 567,
  "total_tokens": 8901234,
  "today_tokens": 123456,
  "quota_used": 4567890,
  "quota_limit": 10000000,
  "quota_type": "tokens",
  "quota_cycle": "monthly",
  "quota_percent": 45.7
}
```

**数据源**：
- `total_requests`: `COUNT(*) FROM request_logs WHERE channel_id = ?`
- `total_tokens`: `SUM(total_tokens) FROM request_logs WHERE channel_id = ?`
- `today_requests/tokens`: 同上 + `AND created_at >= date('now')`
- `quota_used`: 根据 `quota_type` 和 `quota_cycle` 计算当前周期的已用量
  - `tokens` → `SUM(total_tokens)`
  - `requests` → `COUNT(*)`
  - 周期过滤：`daily`=今天, `weekly`=近7天, `monthly`=近30天, `permanent`=全部

### 2. 修改 `GET /admin/channels` 列表接口

在返回的每个渠道 JSON 对象中追加轻量 `stats` 字段：

```json
{
  "id": 1,
  "name": "OpenAI",
  ...existing fields...,
  "stats": {
    "total_requests": 12345,
    "today_requests": 567
  }
}
```

这样前端列表页无需额外 N 次请求。

### 3. 实现位置

- `admin_routes.rs`：新增 `channel_stats` handler
- `admin_routes.rs`：修改 `list_channels` handler，追加 stats 字段
- 直接 SQLx 聚合查询，不新增 models.rs 函数

## 前端改动

### 渠道管理页（`web/src/views/upstream/channel/index.vue`）

- 在现有渠道列表表格中新增 2 列：**累计用量**、**今日用量**
- 单元格格式：`请求数 / Token数`（如 `12,345 / 8.9M`）
- 如果渠道有配额（`quota_type != null`），数字下方显示进度条
  - 进度条颜色：`< 80%` 绿色，`80-100%` 橙色，`> 100%` 红色
- 新增 stats 列的 TypeScript 类型定义

## 技术约束

- 不新增数据库表或字段
- 配额计算周期逻辑复用 `quota.rs` 现有逻辑
- 前端使用 Naive UI `NProgress` 组件
- Token 数格式化：`> 1M` 显示为 `X.XM`，`> 1K` 显示为 `X.XK`
