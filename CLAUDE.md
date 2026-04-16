# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with this repository.

## Project Overview

**CoRide-API** — a lightweight AI API proxy built with Rust (Axum) + Vue 3 (Naive UI / SoybeanAdmin). Allows multiple users to share a group of upstream AI service channels through a unified OpenAI-compatible interface, with per-user quota, rate limiting, and traffic distribution.

## Development Commands

### Backend (Rust)
```bash
cd backend
cargo run              # Dev mode (compile + run)
cargo build            # Build debug
cargo build --release  # Release build
cargo check            # Fast type-check without building
cargo fmt              # Format code
cargo clippy           # Lint
```

### Frontend (Vue 3 + pnpm)
```bash
cd web
pnpm install           # Install dependencies
pnpm dev               # Dev server
pnpm build             # Production build
```

### Docker
```bash
docker compose up -d   # One-click deploy
```

Default admin: `admin` / `admin123`

## Architecture

### Backend (`backend/src/`)

```
main.rs          ── Entry: config → DB → migrations → seed data → AppState → routes
lib.rs           ── AppState struct (config, db pool, rate limiters, http client)
config.rs        ── YAML config loading (config/config.yaml)

router/
  mod.rs         ── Module exports
  proxy_routes.rs ── User-facing proxy endpoints (/v1/chat/completions, etc.)
  admin_routes.rs ── Admin CRUD endpoints (50+ routes under /admin/*)

middleware/
  auth.rs        ── User API Key authentication
  admin_auth.rs  ── Admin JWT authentication
  rate_limit.rs  ── Governor-based QPS/concurrency rate limiting

service/
  proxy.rs       ── HTTP proxy (streaming + non-streaming), build_headers()
  openai.rs      ── OpenAI format adapter
  anthropic.rs   ── Anthropic format adapter
  quota.rs       ── Quota check and deduction
  health.rs      ── Channel health check
  log.rs         ── Request logging

db/
  mod.rs         ── Connection pool init, migration runner
  migrations/    ── 9 SQL migration files (001-009)
  models.rs      ── All struct definitions + CRUD functions (~1200 lines)

state/
  app_state.rs   ── AppState builder

utils/
  token_counter.rs ── Token estimation
```

### Key Backend Data Flow

**Proxy request flow:**
1. `auth.rs` validates API Key → sets `user_id` in context
2. `rate_limit.rs` checks global → user → channel rate limits
3. Quota check (channel → user)
4. Model routing → channel selection (weighted round-robin)
5. **`resolve_app_profile_for_channel()`** — per-channel plan → global plan → legacy `app_profile_id` → None
6. `build_headers(app_profile, ...)` — sets User-Agent + extra headers from traffic plan
7. Forward to upstream (streaming or non-streaming via `proxy.rs`)
8. Parse response → deduct quota → log

**Traffic plan resolution** (`models.rs:resolve_app_profile_for_channel`):
- Queries `traffic_plans` for the channel; falls back to global plan (`channel_id IS NULL`)
- Matches current UTC hour against `traffic_plan_slots` (`start_hour <= now < end_hour`)
- Weighted random selection among matching slots
- Falls back to legacy `channel.app_profile_id` if no plan exists

### Frontend (`web/src/`)

```
views/manage/
  app-profile/     ── App presets (UA + headers templates)
  channel/         ── Channel CRUD with supplier cascader preset
  model/           ── Model list (tree view grouped by channel)
  quota/           ── User quota management
  traffic-plan/    ── Traffic distribution (time-based app routing)
  user/            ── User management
  log/             ── Request log viewer
  stats/           ── Usage statistics with ECharts
  settings/        ── System settings
  ratelimit/       ── Rate limit management

service/api/       ── API request wrappers (Alova-based)
typings/api/       ── TypeScript type definitions
router/elegant/    ── Auto-generated routes
locales/langs/     ── i18n (zh-cn, en-us)
```

### Database Schema (SQLite)

| Table | Purpose |
|-------|---------|
| users | User accounts with API Keys |
| channels | Upstream AI service providers |
| models | Model name mapping (source → proxy) |
| app_profiles | App disguise presets (User-Agent + headers) |
| traffic_plans | Time-based traffic distribution (NULL channel_id = global) |
| traffic_plan_slots | Per-plan time slots with app profile weights |
| quotas | Per-user quota (requests or tokens) |
| ratelimit_configs | Rate limit rules (global/channel/user) |
| request_logs | Request audit trail |

### Response Format

Admin API responses use: `{"code": 0, "message": "ok", "data": ...}`
Error responses use: `{"code": <HTTP status>, "message": "<error text>"}`

### Key Patterns

- All admin routes require JWT auth via `admin_auth::admin_auth_middleware`
- Admin handlers use `ok_response()` / `error_response()` helpers
- Model functions use SQLx directly (no ORM), following existing naming conventions in `models.rs`
- Frontend uses Naive UI components; always import specific components rather than using global registration
- Route files follow `elegant-router` convention — add routes to `routes.ts` and i18n entries to locale files
