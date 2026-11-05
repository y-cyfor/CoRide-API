# API Key 独立配额（渠道级）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 扩展 `quotas` 表增加 `channel_id` 字段，支持"用户 × 渠道"维度的配额限制，优先级：渠道配额 > 用户总配额。

**Architecture:** 新增 SQL 迁移添加 `channel_id` 列。修改 `check_user_quota` 和 `deduct_user_quota` 接受 `channel_id` 参数，优先匹配渠道配额，回退到用户级。修改 proxy 调用方传入 channel ID。前端配额管理增加渠道选择。

**Tech Stack:** Rust (Axum + SQLx + SQLite), Vue 3 + Naive UI, TypeScript

---

### Task 1: 数据库迁移 — quotas 表增加 channel_id 列

**Files:**
- Create: `backend/src/db/migrations/011_quotas_channel_id.sql`

- [ ] **Step 1: 创建迁移文件**

```sql
-- Migration 011: Add channel_id to quotas table for per-channel user quotas

ALTER TABLE quotas ADD COLUMN channel_id INTEGER;
```

- [ ] **Step 2: 提交**

```bash
git add backend/src/db/migrations/011_quotas_channel_id.sql
git commit -m "feat: quotas 表增加 channel_id 列支持渠道级配额"
```

---

### Task 2: 后端 — 修改配额检查/扣除逻辑支持 channel_id

**Files:**
- Modify: `backend/src/db/models.rs` — 修改 `get_active_quotas` 或新增 `get_quota_for_channel`
- Modify: `backend/src/service/quota.rs` — 修改 `check_user_quota` 和 `deduct_user_quota`

- [ ] **Step 1: 在 models.rs 中新增查询函数**

在 `models.rs` 中添加：

```rust
/// Get active quota for a user on a specific channel.
/// Returns the channel-level quota if exists, otherwise falls back to user-level (channel_id IS NULL).
pub async fn get_user_quota_for_channel(
    pool: &SqlitePool,
    user_id: i64,
    channel_id: i64,
) -> Result<Option<Quota>, sqlx::Error> {
    // Try channel-level quota first
    let channel_quota: Option<Quota> = sqlx::query_as(
        "SELECT * FROM quotas WHERE user_id = ? AND channel_id = ? AND enabled = 1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(channel_id)
    .fetch_optional(pool)
    .await?;

    if channel_quota.is_some() {
        return Ok(channel_quota);
    }

    // Fallback to user-level quota (channel_id IS NULL)
    sqlx::query_as(
        "SELECT * FROM quotas WHERE user_id = ? AND channel_id IS NULL AND enabled = 1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// Get all active quotas for a user (unchanged, for admin display).
/// Existing `get_active_quotas` remains for backward compatibility.
```

- [ ] **Step 2: 修改 quota.rs 的 check_user_quota**

Replace the existing `check_user_quota` function:

```rust
/// Check if a user has remaining quota for a specific channel.
/// Checks channel-level quota first, falls back to user-level quota.
pub async fn check_user_quota(
    pool: &SqlitePool,
    user_id: i64,
    channel_id: i64,
    tokens: i32,
) -> Result<(), QuotaError> {
    let quota = match models::get_user_quota_for_channel(pool, user_id, channel_id).await? {
        Some(q) => q,
        None => return Ok(()), // No quota restriction = allowed
    };

    // Reset used if cycle expired
    reset_quota_if_expired(pool, &quota).await?;

    match quota.quota_type.as_str() {
        "requests" => {
            if quota.used >= quota.total_limit {
                return Err(QuotaError::Exceeded(format!("{} requests", quota.total_limit)));
            }
        }
        "tokens" => {
            if quota.used + (tokens as i64) > quota.total_limit {
                return Err(QuotaError::Exceeded(format!("{} tokens", quota.total_limit)));
            }
        }
        _ => {}
    }

    Ok(())
}
```

- [ ] **Step 3: 修改 quota.rs 的 deduct_user_quota**

Replace the existing `deduct_user_quota` function:

```rust
/// Deduct user quota for a specific channel after a successful request.
/// Only deducts from the applicable quota (channel-level or user-level).
pub async fn deduct_user_quota(
    pool: &SqlitePool,
    user_id: i64,
    channel_id: i64,
    tokens: i32,
) -> Result<(), QuotaError> {
    let quota = match models::get_user_quota_for_channel(pool, user_id, channel_id).await? {
        Some(q) => q,
        None => return Ok(()), // No quota to deduct from
    };

    match quota.quota_type.as_str() {
        "requests" => {
            models::increment_quota_used(pool, quota.id, 1).await?;
        }
        "tokens" => {
            models::increment_quota_used(pool, quota.id, tokens as i64).await?;
        }
        _ => {}
    }

    Ok(())
}
```

- [ ] **Step 4: 编译验证**

```bash
cd backend && cargo check
```

- [ ] **Step 5: 提交**

```bash
git add backend/src/db/models.rs backend/src/service/quota.rs
git commit -m "feat: 配额检查/扣除逻辑支持渠道维度，优先渠道配额回退到用户配额"
```

---

### Task 3: 后端 — 修改 proxy 调用方传入 channel ID

**Files:**
- Modify: `backend/src/router/proxy_routes.rs`

- [ ] **Step 1: 修改 quota 调用点**

In `proxy_routes.rs`, find all calls to `quota::check_user_quota` and `quota::deduct_user_quota` and add `channel.id` as the third argument.

There are 4 call sites:

1. Line ~178: `quota::check_user_quota(pool, user_id, estimated_tokens)` → `quota::check_user_quota(pool, user_id, channel.id, estimated_tokens)`

2. Line ~206: `quota::deduct_user_quota(pool, user_id, estimated_tokens)` → `quota::deduct_user_quota(pool, user_id, channel.id, estimated_tokens)`

3. Line ~256: `quota::deduct_user_quota(pool, user_id, result.total_tokens)` → `quota::deduct_user_quota(pool, user_id, channel.id, result.total_tokens)`

- [ ] **Step 2: 编译验证**

```bash
cd backend && cargo check
```

- [ ] **Step 3: 提交**

```bash
git add backend/src/router/proxy_routes.rs
git commit -m "fix: proxy 调用配额检查/扣除时传入渠道 ID"
```

---

### Task 4: 后端 — 配额 CRUD 支持 channel_id

**Files:**
- Modify: `backend/src/router/admin_routes.rs`

- [ ] **Step 1: 修改 CreateQuotaRequest**

Add `channel_id` field to `CreateQuotaRequest`:

```rust
#[derive(Debug, Deserialize)]
pub struct CreateQuotaRequest {
    pub user_id: i64,
    pub quota_type: String,
    pub total_limit: i64,
    pub cycle: String,
    pub channel_id: Option<i64>,
}
```

- [ ] **Step 2: 修改 create_quota handler**

Update the handler to include `channel_id`:

```rust
pub async fn create_quota(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateQuotaRequest>,
) -> Response {
    let pool = &state.db;
    let (period_start, period_end) = crate::service::quota::init_quota_period(&req.cycle);

    let result = sqlx::query(
        "INSERT INTO quotas (user_id, quota_type, total_limit, cycle, period_start, period_end, channel_id, enabled) VALUES (?, ?, ?, ?, ?, ?, ?, true)",
    )
    .bind(req.user_id)
    .bind(&req.quota_type)
    .bind(req.total_limit)
    .bind(&req.cycle)
    .bind(period_start)
    .bind(period_end)
    .bind(req.channel_id)
    .execute(pool);

    match result.await {
        Ok(r) => ok_response(serde_json::json!({ "id": r.last_insert_rowid() })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}
```

- [ ] **Step 3: 编译验证**

```bash
cd backend && cargo check
```

- [ ] **Step 4: 提交**

```bash
git add backend/src/router/admin_routes.rs
git commit -m "feat: 创建配额接口支持 channel_id 参数"
```

---

### Task 5: 前端 — TypeScript 类型 + 配额管理增加渠道选择

**Files:**
- Modify: `web/src/typings/api/liteproxy.d.ts` — Quota 类型
- Modify: `web/src/service/api/quota.ts` — API 函数
- Modify: `web/src/views/control/quota/index.vue` — 增加渠道选择

- [ ] **Step 1: 扩展 Quota 类型**

In `liteproxy.d.ts`, update Quota interface:

```typescript
interface Quota {
  id: number;
  user_id: number;
  quota_type: string;
  total_limit: number;
  used: number;
  cycle: string;
  channel_id?: number | null;
  period_start?: string;
  period_end?: string;
  enabled: boolean;
  created_at: string;
}

interface CreateQuotaParams {
  user_id: number;
  quota_type: string;
  total_limit: number;
  cycle: string;
  channel_id?: number | null;
}
```

- [ ] **Step 2: 新增 API 函数获取渠道列表（用于下拉）**

In `channel.ts`, add a simple list function if not already exists:

```typescript
/** Get all channels (for dropdown) */
export function fetchAllChannels() {
  return request<Array<{ id: number; name: string }>>({
    url: '/admin/channels',
    method: 'get',
    params: { page: 1, page_size: 1000 }
  });
}
```

- [ ] **Step 3: 修改配额管理页面**

In `web/src/views/control/quota/index.vue`, add a channel selector to the create/edit form:

Add import: `import { fetchAllChannels } from '@/service/api/channel';`

Add state: `const channelOptions = ref<Array<{ label: string; value: number }>>([]);`

In `formModel`, add: `channel_id: null as number | null | undefined,`

Load channels on mount:
```typescript
async function loadChannels() {
  const { data } = await fetchAllChannels();
  if (data) {
    const items = (data as any).items || data;
    channelOptions.value = items.map((c: any) => ({ label: c.name, value: c.id }));
  }
}
```

Add form item before the quota type selector:
```html
<NFormItem label="渠道">
  <NSelect v-model:value="formModel.channel_id" :options="channelOptions"
    placeholder="不选 = 用户级总配额" clearable />
</NFormItem>
```

- [ ] **Step 4: 配额列表增加渠道列**

In the table columns, add a channel column showing the channel name or "全部":

```typescript
{ title: '渠道', key: 'channel', width: 100,
  render: row => row.channel_id ? `渠道#${row.channel_id}` : '全部' }
```

- [ ] **Step 5: 提交**

```bash
git add web/src/typings/api/liteproxy.d.ts web/src/service/api/quota.ts web/src/service/api/channel.ts web/src/views/control/quota/index.vue
git commit -m "feat: 配额管理前端增加渠道选择和列表显示"
```

---

## Self-Review

1. **Spec coverage**: 所有需求覆盖：
   - ✅ 数据库迁移添加 channel_id 列
   - ✅ 配额检查/扣除逻辑支持渠道维度
   - ✅ Proxy 调用方传入 channel ID
   - ✅ 配额 CRUD 支持 channel_id
   - ✅ 前端类型 + 渠道选择 + 列表显示

2. **Placeholder scan**: 无 TBD/TODO，所有代码步骤包含完整代码。

3. **Type consistency**: `channel_id` 类型在后端 (`Option<i64>`) 和前端 (`number | null | undefined`) 一致。

4. **Scope check**: 5 个任务，每个独立可提交。未引入无关重构。
