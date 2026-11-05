# IP 访问控制 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现全局黑名单 + 用户白名单的 IP 访问控制。全局黑名单优先级最高，用户白名单限制特定用户只能从指定 IP 访问。

**Architecture:** 两张新表（ip_blacklist, ip_whitelist），在 auth 中间件注入 IP 校验逻辑。系统设置页新增黑名单配置，用户管理页新增白名单配置。

**Tech Stack:** Rust (Axum + SQLx + SQLite), Vue 3 + Naive UI, TypeScript

---

### Task 1: 数据库迁移 — 创建 IP 黑名单/白名单表

**Files:**
- Create: `backend/src/db/migrations/012_ip_access_control.sql`

- [ ] **Step 1: 创建迁移文件**

```sql
-- Migration 012: IP access control tables

-- Global IP blacklist (system-level, applies to all users)
CREATE TABLE IF NOT EXISTS ip_blacklist (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ip_address TEXT NOT NULL UNIQUE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Per-user IP whitelist (empty = no restriction for that user)
CREATE TABLE IF NOT EXISTS ip_whitelist (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    ip_address TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, ip_address)
);
```

- [ ] **Step 2: 提交**

```bash
git add backend/src/db/migrations/012_ip_access_control.sql
git commit -m "feat: 新增 IP 访问控制表（全局黑名单 + 用户白名单）"
```

---

### Task 2: 后端 — IP 校验中间件

**Files:**
- Create: `backend/src/middleware/ip_filter.rs`
- Modify: `backend/src/db/models.rs` — 新增 IP 查询函数
- Modify: `backend/src/lib.rs` 或 `backend/src/state/app_state.rs` — 注册中间件

- [ ] **Step 1: 在 models.rs 中添加 IP 查询函数**

```rust
/// Check if an IP is in the global blacklist.
pub async fn is_ip_blacklisted(pool: &SqlitePool, ip: &str) -> Result<bool, sqlx::Error> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ip_blacklist WHERE ip_address = ?",
    )
    .bind(ip)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

/// Check if a user has an IP whitelist and if the given IP is in it.
/// Returns Ok(true) if whitelist exists and IP matches.
/// Returns Ok(false) if whitelist exists but IP doesn't match.
/// Returns Ok(None) if user has no whitelist (no restriction).
pub async fn check_user_ip_whitelist(
    pool: &SqlitePool,
    user_id: i64,
    ip: &str,
) -> Result<Option<bool>, sqlx::Error> {
    // Check if user has any whitelist entries
    let has_whitelist: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ip_whitelist WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    if has_whitelist == 0 {
        return Ok(None); // No restriction
    }

    // Check if IP is in whitelist
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ip_whitelist WHERE user_id = ? AND ip_address = ?",
    )
    .bind(user_id)
    .bind(ip)
    .fetch_one(pool)
    .await?;

    Ok(Some(count > 0))
}
```

- [ ] **Step 2: 创建 ip_filter.rs 中间件**

Create `backend/src/middleware/ip_filter.rs`:

```rust
use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::AppState;

/// Extract client IP from request headers (X-Real-IP > X-Forwarded-For > remote_addr).
fn extract_client_ip(req: &Request<Body>) -> String {
    // Try X-Real-IP first (most reliable single IP)
    if let Some(real_ip) = req.headers().get("X-Real-IP") {
        if let Ok(ip) = real_ip.to_str() {
            return ip.trim().to_string();
        }
    }

    // Try X-Forwarded-For (take first IP in comma-separated list)
    if let Some(xff) = req.headers().get("X-Forwarded-For") {
        if let Ok(xff_str) = xff.to_str() {
            if let Some(first_ip) = xff_str.split(',').next() {
                let trimmed = first_ip.trim().to_string();
                if !trimmed.is_empty() {
                    return trimmed;
                }
            }
        }
    }

    // Fallback to remote_addr
    req.extensions()
        .get::<std::net::SocketAddr>()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// IP filter middleware: check global blacklist + user whitelist.
pub async fn ip_filter(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let ip = extract_client_ip(&req);

    // Check global blacklist
    match crate::db::models::is_ip_blacklisted(&state.db, &ip).await {
        Ok(true) => {
            tracing::warn!(ip = ip, "Blocked request: IP in global blacklist");
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(false) => {}
        Err(e) => {
            tracing::error!(error = %e, "Failed to check IP blacklist, allowing request");
        }
    }

    // Check user whitelist (only if user_id is in extensions, set by auth middleware)
    if let Some(user_id) = req.extensions().get::<i64>().copied() {
        match crate::db::models::check_user_ip_whitelist(&state.db, user_id, &ip).await {
            Ok(Some(false)) => {
                // User has whitelist and IP is not in it
                tracing::warn!(ip = ip, user_id = user_id, "Blocked request: IP not in user whitelist");
                return Err(StatusCode::FORBIDDEN);
            }
            Ok(Some(true)) | Ok(None) => {} // IP matches whitelist or no restriction
            Err(e) => {
                tracing::error!(error = %e, "Failed to check IP whitelist, allowing request");
            }
        }
    }

    Ok(next.run(req).await)
}
```

- [ ] **Step 3: 注册中间件**

In `backend/src/middleware/mod.rs`, add:
```rust
pub mod ip_filter;
```

In `backend/src/main.rs`, add the ip_filter layer to proxy_routes (after auth):
```rust
.layer(from_fn_with_state(state.clone(), ip_filter::ip_filter))
```

The ip_filter must run AFTER auth (so user_id is available) but BEFORE rate_limit/quota.

- [ ] **Step 4: 编译验证**

```bash
cd backend && cargo check
```

- [ ] **Step 5: 提交**

```bash
git add backend/src/middleware/ip_filter.rs backend/src/middleware/mod.rs backend/src/db/models.rs backend/src/main.rs
git commit -m "feat: IP 访问控制中间件（全局黑名单 + 用户白名单）"
```

---

### Task 3: 后端 — IP 黑名单/白名单 CRUD 接口

**Files:**
- Modify: `backend/src/router/admin_routes.rs`

- [ ] **Step 1: 添加黑名单请求 DTO**

In `admin_routes.rs`, add request structs:

```rust
#[derive(Debug, Deserialize)]
pub struct AddBlacklistRequest {
    pub ip_address: String,
}

#[derive(Debug, Deserialize)]
pub struct AddWhitelistRequest {
    pub user_id: i64,
    pub ip_address: String,
}
```

- [ ] **Step 2: 添加黑名单 CRUD handlers**

```rust
/// GET /admin/ip/blacklist
pub async fn list_blacklist(
    State(state): State<Arc<AppState>>,
) -> Response {
    let pool = &state.db;
    match sqlx::query_as::<_, (i64, String, String)>("SELECT id, ip_address, created_at FROM ip_blacklist ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
    {
        Ok(entries) => ok_response(entries.iter().map(|(id, ip, created)| {
            serde_json::json!({"id": id, "ip_address": ip, "created_at": created})
        }).collect::<Vec<_>>()),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// POST /admin/ip/blacklist
pub async fn add_blacklist(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddBlacklistRequest>,
) -> Response {
    let pool = &state.db;
    match sqlx::query("INSERT INTO ip_blacklist (ip_address) VALUES (?)")
        .bind(&req.ip_address)
        .execute(pool)
        .await
    {
        Ok(r) => ok_response(serde_json::json!({"id": r.last_insert_rowid()})),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// DELETE /admin/ip/blacklist/{id}
pub async fn delete_blacklist(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = &state.db;
    match sqlx::query("DELETE FROM ip_blacklist WHERE id = ?").bind(id).execute(pool).await {
        Ok(_) => ok_response(serde_json::json!({"deleted": true})),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}
```

- [ ] **Step 3: 添加用户白名单 CRUD handlers**

```rust
/// GET /admin/ip/whitelist/{user_id}
pub async fn list_user_whitelist(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i64>,
) -> Response {
    let pool = &state.db;
    match sqlx::query_as::<_, (i64, i64, String, String)>("SELECT id, user_id, ip_address, created_at FROM ip_whitelist WHERE user_id = ? ORDER BY created_at")
        .bind(user_id)
        .fetch_all(pool)
        .await
    {
        Ok(entries) => ok_response(entries.iter().map(|(id, uid, ip, created)| {
            serde_json::json!({"id": id, "user_id": uid, "ip_address": ip, "created_at": created})
        }).collect::<Vec<_>>()),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// POST /admin/ip/whitelist
pub async fn add_whitelist(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddWhitelistRequest>,
) -> Response {
    let pool = &state.db;
    match sqlx::query("INSERT OR IGNORE INTO ip_whitelist (user_id, ip_address) VALUES (?, ?)")
        .bind(req.user_id)
        .bind(&req.ip_address)
        .execute(pool)
        .await
    {
        Ok(r) => ok_response(serde_json::json!({"id": r.last_insert_rowid()})),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// DELETE /admin/ip/whitelist/{id}
pub async fn delete_whitelist(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = &state.db;
    match sqlx::query("DELETE FROM ip_whitelist WHERE id = ?").bind(id).execute(pool).await {
        Ok(_) => ok_response(serde_json::json!({"deleted": true})),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}
```

- [ ] **Step 4: 注册路由**

In `main.rs`, add routes under `admin_protected`:

```rust
// IP access control
.route("/admin/ip/blacklist", get(admin_routes::list_blacklist))
.route("/admin/ip/blacklist", post(admin_routes::add_blacklist))
.route("/admin/ip/blacklist/{id}", delete(admin_routes::delete_blacklist))
.route("/admin/ip/whitelist/{user_id}", get(admin_routes::list_user_whitelist))
.route("/admin/ip/whitelist", post(admin_routes::add_whitelist))
.route("/admin/ip/whitelist/{id}", delete(admin_routes::delete_whitelist))
```

- [ ] **Step 5: 编译验证**

```bash
cd backend && cargo check
```

- [ ] **Step 6: 提交**

```bash
git add backend/src/router/admin_routes.rs backend/src/main.rs
git commit -m "feat: IP 黑白名单 CRUD 接口"
```

---

### Task 4: 前端 — 系统设置页添加黑名单管理 + 用户管理页添加白名单

**Files:**
- Modify: `web/src/views/settings/index.vue` — 添加黑名单管理
- Modify: `web/src/views/control/user/index.vue` — 添加用户白名单管理
- Modify: `web/src/service/api/` — 新增 ip.ts API 封装
- Modify: `web/src/typings/api/liteproxy.d.ts` — 新增类型

- [ ] **Step 1: 新增类型定义**

In `liteproxy.d.ts`, add:

```typescript
namespace IpAccess {
  interface BlacklistEntry {
    id: number;
    ip_address: string;
    created_at: string;
  }

  interface WhitelistEntry {
    id: number;
    user_id: number;
    ip_address: string;
    created_at: string;
  }

  interface AddBlacklistParams {
    ip_address: string;
  }

  interface AddWhitelistParams {
    user_id: number;
    ip_address: string;
  }
}
```

- [ ] **Step 2: 新增 API 封装**

Create `web/src/service/api/ip.ts`:

```typescript
import { request } from '../request';

// Blacklist
export function fetchBlacklistList() {
  return request<Api.IpAccess.BlacklistEntry[]>({ url: '/admin/ip/blacklist', method: 'get' });
}
export function fetchAddBlacklist(data: Api.IpAccess.AddBlacklistParams) {
  return request<{ id: number }>({ url: '/admin/ip/blacklist', method: 'post', data });
}
export function fetchDeleteBlacklist(id: number) {
  return request<{ deleted: boolean }>({ url: `/admin/ip/blacklist/${id}`, method: 'delete' });
}

// Whitelist
export function fetchUserWhitelist(userId: number) {
  return request<Api.IpAccess.WhitelistEntry[]>({ url: `/admin/ip/whitelist/${userId}`, method: 'get' });
}
export function fetchAddWhitelist(data: Api.IpAccess.AddWhitelistParams) {
  return request<{ id: number }>({ url: '/admin/ip/whitelist', method: 'post', data });
}
export function fetchDeleteWhitelist(id: number) {
  return request<{ deleted: boolean }>({ url: `/admin/ip/whitelist/${id}`, method: 'delete' });
}
```

- [ ] **Step 3: 系统设置页添加黑名单**

Read `web/src/views/settings/index.vue` first. Add a blacklist management section:
- NInput + NButton to add IP
- NTable showing blacklist entries with delete buttons
- Load on mount via fetchBlacklistList

- [ ] **Step 4: 用户管理页添加白名单**

Read `web/src/views/control/user/index.vue` first. Add a per-user whitelist section:
- In the user detail/edit modal, add a whitelist management area
- NInput + NButton to add IP for the selected user
- NTag list showing whitelist entries with delete
- Load when user detail is opened via fetchUserWhitelist(user_id)

- [ ] **Step 5: 提交**

```bash
git add web/src/typings/api/liteproxy.d.ts web/src/service/api/ip.ts web/src/views/settings/index.vue web/src/views/control/user/index.vue
git commit -m "feat: 前端系统设置和用户管理页增加 IP 黑白名单管理"
```

---

## Self-Review

1. **Spec coverage**: 全部覆盖：
   - ✅ 数据库迁移创建两张表
   - ✅ IP 校验中间件（黑名单优先 > 用户白名单）
   - ✅ IP 提取逻辑（X-Real-IP > X-Forwarded-For > remote_addr）
   - ✅ CRUD 接口
   - ✅ 前端类型 + API + 两处 UI

2. **Placeholder scan**: 无 TBD/TODO。

3. **Type consistency**: `ip_address` 在前后端均为 string。`Option<i64>` / `number | null` 一致。

4. **Scope check**: 4 个任务，独立可提交。未引入无关重构。
