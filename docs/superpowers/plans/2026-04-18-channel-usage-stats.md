# 渠道用量统计面板 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在渠道管理列表页显示每个渠道的请求数和 Token 消耗统计（累计 + 今日），纯 SQL 聚合，不新增数据库表。

**Architecture:** 修改 `list_channels` handler，在返回渠道列表时对每个渠道执行轻量 SQL 聚合查询，将 stats 字段注入 JSON 响应。新增 `GET /admin/channels/{id}/stats` 接口提供单个渠道的详细统计。前端在表格新增 2 列展示。

**Tech Stack:** Rust (Axum + SQLx), Vue 3 + Naive UI, TypeScript

---

### Task 1: 后端 — list_channels 注入请求统计

**Files:**
- Modify: `backend/src/router/admin_routes.rs:328-349` (list_channels handler)

- [ ] **Step 1: 修改 list_channels 注入 stats**

在 `admin_routes.rs` 的 `list_channels` handler 中，对每个渠道追加 stats 字段。当前代码直接返回 `channels` JSON，需要改为遍历 channels 并对每个渠道执行聚合查询：

```rust
pub async fn list_channels(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
) -> Response {
    let pool = &state.db;
    let page = pagination.page.unwrap_or(1);
    let page_size = pagination.page_size.unwrap_or(20);

    // Get total count
    let total: i64 = match sqlx::query_scalar("SELECT COUNT(*) FROM channels").fetch_one(pool).await {
        Ok(t) => t,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    match models::list_channels(pool, page, page_size).await {
        Ok(channels) => {
            // Enrich each channel with request stats from request_logs
            let mut enriched = Vec::new();
            for ch in channels {
                let ch_json = serde_json::to_value(&ch).unwrap_or_default();
                if let Some(mut obj) = ch_json.as_object().cloned() {
                    let stats: Option<(i64, i64)> = sqlx::query_as(
                        "SELECT COUNT(*), COALESCE(SUM(total_tokens), 0) FROM request_logs WHERE channel_id = ?",
                    )
                    .bind(ch.id)
                    .fetch_optional(pool)
                    .await
                    .ok()
                    .flatten();

                    let today_stats: Option<(i64, i64)> = sqlx::query_as(
                        "SELECT COUNT(*), COALESCE(SUM(total_tokens), 0) FROM request_logs WHERE channel_id = ? AND created_at >= date('now')",
                    )
                    .bind(ch.id)
                    .fetch_optional(pool)
                    .await
                    .ok()
                    .flatten();

                    let (total_req, total_tok) = stats.unwrap_or((0, 0));
                    let (today_req, today_tok) = today_stats.unwrap_or((0, 0));

                    obj.insert("stats".to_string(), serde_json::json!({
                        "total_requests": total_req,
                        "total_tokens": total_tok,
                        "today_requests": today_req,
                        "today_tokens": today_tok,
                    }));

                    enriched.push(serde_json::Value::Object(obj));
                }
            }

            ok_response(serde_json::json!({
                "items": enriched,
                "total": total,
            }))
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}
```

- [ ] **Step 2: 编译验证**

```bash
cd backend && cargo check
```

预期：无编译错误。

- [ ] **Step 3: 提交**

```bash
git add backend/src/router/admin_routes.rs
git commit -m "feat: 渠道列表注入请求统计（累计+今日）"
```

---

### Task 2: 后端 — 新增 `GET /admin/channels/{id}/stats` 详细统计接口

**Files:**
- Modify: `backend/src/router/admin_routes.rs` (新增 handler + 路由注册)
- Modify: `backend/src/main.rs` (注册新路由)

- [ ] **Step 1: 添加 channel_stats handler**

在 `admin_routes.rs` 中，在所有 channel handlers 之后（约第 473 行 `test_channel` 之后）添加：

```rust
/// GET /admin/channels/{id}/stats — detailed usage stats for a single channel
pub async fn channel_stats(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = &state.db;

    // Verify channel exists
    match models::get_channel_by_id(pool, id).await {
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "Channel not found"),
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        _ => {}
    }

    // Total stats
    let total_stats: Option<(i64, i64)> = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(total_tokens), 0) FROM request_logs WHERE channel_id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    // Today stats
    let today_stats: Option<(i64, i64)> = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(total_tokens), 0) FROM request_logs WHERE channel_id = ? AND created_at >= date('now')",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let (total_req, total_tok) = total_stats.unwrap_or((0, 0));
    let (today_req, today_tok) = today_stats.unwrap_or((0, 0));

    ok_response(serde_json::json!({
        "total_requests": total_req,
        "total_tokens": total_tok,
        "today_requests": today_req,
        "today_tokens": today_tok,
    }))
}
```

- [ ] **Step 2: 注册路由**

在 `main.rs` 的 admin_protected Router 中，约在第 107 行（`/admin/channels/{id}/test` 之后）添加：

```rust
.route("/admin/channels/{id}/stats", get(admin_routes::channel_stats))
```

- [ ] **Step 3: 编译验证**

```bash
cd backend && cargo check
```

- [ ] **Step 4: 提交**

```bash
git add backend/src/router/admin_routes.rs backend/src/main.rs
git commit -m "feat: 新增渠道详细统计接口 GET /admin/channels/{id}/stats"
```

---

### Task 3: 前端 — TypeScript 类型 + API 封装

**Files:**
- Modify: `web/src/typings/api/liteproxy.d.ts` (Channel 类型)
- Modify: `web/src/service/api/channel.ts` (新增 API 函数)

- [ ] **Step 1: 扩展 Channel 类型**

在 `web/src/typings/api/liteproxy.d.ts` 的 `Channel` interface 中添加 `stats` 字段：

```typescript
interface Channel {
  id: number;
  name: string;
  type: string;
  base_url: string;
  api_keys: string;
  custom_headers?: string;
  status: string;
  weight: number;
  timeout: number;
  retry_count: number;
  quota_type?: string;
  quota_limit?: number;
  quota_used: number;
  quota_cycle?: string;
  quota_period_start?: string;
  quota_period_end?: string;
  app_profile_id?: number;
  created_at: string;
  updated_at: string;
  stats?: {
    total_requests: number;
    total_tokens: number;
    today_requests: number;
    today_tokens: number;
  };
}
```

- [ ] **Step 2: 新增 API 函数**

在 `web/src/service/api/channel.ts` 末尾添加：

```typescript
/** Get channel usage stats */
export function fetchChannelStats(id: number) {
  return request<{
    total_requests: number;
    total_tokens: number;
    today_requests: number;
    today_tokens: number;
  }>({
    url: `/admin/channels/${id}/stats`,
    method: 'get'
  });
}
```

- [ ] **Step 3: 验证前端编译**

```bash
cd web && pnpm run typecheck
```

- [ ] **Step 4: 提交**

```bash
git add web/src/typings/api/liteproxy.d.ts web/src/service/api/channel.ts
git commit -m "feat: 前端类型定义 + API 封装适配渠道统计"
```

---

### Task 4: 前端 — 渠道列表新增用量列

**Files:**
- Modify: `web/src/views/upstream/channel/index.vue`

- [ ] **Step 1: 新增 token 格式化函数**

在 `columns` 定义之前（约第 195 行之前）添加：

```typescript
function formatTokenNum(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toLocaleString();
}
```

- [ ] **Step 2: 在 columns 中插入两列**

在"健康"列之后（约第 215 行之后），插入两列：

```typescript
{
  title: '累计用量',
  key: 'total_usage',
  width: 140,
  render: row => {
    const stats = (row as any).stats;
    const reqs = stats?.total_requests ?? 0;
    const toks = stats?.total_tokens ?? 0;
    return h('div', { style: 'font-size: 12px' }, [
      h('div', null, `${reqs.toLocaleString()} 次`),
      h('div', { style: 'color: #999; font-size: 11px' }, `${formatTokenNum(toks)} tokens`)
    ]);
  }
},
{
  title: '今日用量',
  key: 'today_usage',
  width: 140,
  render: row => {
    const stats = (row as any).stats;
    const reqs = stats?.today_requests ?? 0;
    const toks = stats?.today_tokens ?? 0;
    return h('div', { style: 'font-size: 12px' }, [
      h('div', null, `${reqs.toLocaleString()} 次`),
      h('div', { style: 'color: #999; font-size: 11px' }, `${formatTokenNum(toks)} tokens`)
    ]);
  }
},
```

- [ ] **Step 3: 验证前端编译**

```bash
cd web && pnpm run typecheck
```

- [ ] **Step 4: 提交**

```bash
git add web/src/views/upstream/channel/index.vue
git commit -m "feat: 渠道列表新增累计用量和今日用量列"
```

---

## Self-Review

1. **Spec coverage**: 所有需求已覆盖：
   - ✅ 渠道列表注入统计（后端 Task 1）
   - ✅ 详细统计接口（后端 Task 2）
   - ✅ 类型定义 + API 封装（前端 Task 3）
   - ✅ 用量列展示（前端 Task 4）

2. **Placeholder scan**: 无 TBD/TODO，所有代码步骤包含完整代码。

3. **Type consistency**: 后端 stats JSON 字段名 (`total_requests`, `total_tokens`, `today_requests`, `today_tokens`) 与前端类型定义完全一致。

4. **Scope check**: 专注在 4 个任务内，未引入无关重构。
