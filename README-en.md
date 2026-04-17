# CoRide-API

> A lightweight AI API proxy service — multi-model carpooling

<div align="center">

[中文文档](README.md)

[![License](https://img.shields.io/badge/license-Apache--2.0-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)
[![Vue](https://img.shields.io/badge/vue-3-green.svg)](https://vuejs.org/)
[![Docker](https://img.shields.io/badge/Docker-cyfor%2Fcoride--api-blue)](https://hub.docker.com/r/cyfor/coride-api)

</div>

> This document is translated from the Chinese version. For the original, refer to [中文文档](README.md).

---

## Introduction

CoRide-API is a lightweight AI API proxy management tool built with Rust + Vue 3. It allows multiple users to share a set of upstream AI service channels (such as OpenAI, Anthropic, Alibaba Cloud Tongyi Qianwen, Zhipu AI, Kimi, etc.) through a unified OpenAI-compatible interface.

**Core Business Scenarios:**

- Administrators centrally manage API Keys and configurations of multiple AI service channels
- Multiple users access the proxy service through their individual API Keys
- Per-user quota limits (requests / tokens) and rate control
- Model binding per user for fine-grained access control
- Time-based, weighted request routing to different app presets (UA + header伪装)
- Full request logging and statistics for operational monitoring

**Project Positioning:** A personal/small-team carpooling tool with no commercial features, focused on simplicity, efficiency, and easy deployment.

---

## About Vibe Coding

This project is entirely developed through **Vibe Coding** — the developer describes requirements in natural language, and AI writes the code.

| Item | Tool/Model |
|------|-----------|
| Programming Tools | [Claude Code](https://github.com/anthropics/claude-code), [OpenCode](https://github.com/opencode-ai/opencode) |
| LLMs | Qwen 3.6-plus, Xiaomi MiMo v2 Pro, Zhipu GLM-5.1 |

---

## Installation & Usage

### Prerequisites

- **Rust 1.88+** (backend development)
- **Node.js 20+** + pnpm (frontend development)
- **Docker & Docker Compose** (optional, recommended)

### Method 1: Deploy from Docker Hub Image (Recommended)

```bash
docker pull cyfor/coride-api:latest

# Create docker-compose.yml and .env
mkdir -p /opt/coride
cat > /opt/coride/.env << 'EOF'
CORIDE_ADMIN_USERNAME=admin
CORIDE_ADMIN_PASSWORD=your-password
CORIDE_JWT_SECRET=your-random-secret
CORIDE_LOG_LEVEL=info
EOF

cat > /opt/coride/docker-compose.yml << 'EOF'
services:
  coride-api:
    image: cyfor/coride-api:latest
    container_name: coride-api
    restart: unless-stopped
    ports:
      - "80:80"
    volumes:
      - ./data:/app/data
      - ./logs:/app/log
    env_file:
      - .env
    environment:
      - CORIDE_DB_PATH=/app/data/coride.db
EOF

cd /opt/coride && docker compose up -d
```

After startup, visit `http://server-ip` and log in with the admin credentials set in `.env`.

> **Port Notes:**
> - Inside Docker container: nginx listens on port **80** (reverse proxy to backend port 8000)
> - `"80:80"` format is `"host-port:container-port"`
> - To change the external port, modify the first number, e.g., `"8080:80"` → `http://IP:8080`
> - **BT Panel deployment**: set container port to **80**, customize host port, then reverse proxy to `http://127.0.0.1:host-port`
> - ⚠️ Port 9527 is only for Vite dev environment, not used in Docker

### Method 2: Development Environment

```bash
# Backend
cd backend && cargo run

# Frontend (new terminal)
cd web && pnpm install && pnpm dev
```

Default admin: `admin` / `admin123`

> In production, use environment variables `CORIDE_ADMIN_PASSWORD` and `CORIDE_JWT_SECRET` to override defaults.

---

## Project Structure

```
CoRide-API/
├── backend/
│   ├── src/
│   │   ├── main.rs                  # Entry: routes, background tasks, graceful shutdown
│   │   ├── lib.rs                   # Library entry: AppState
│   │   ├── config.rs                # Config loading (YAML + env override)
│   │   ├── state/app_state.rs       # App state builder
│   │   ├── db/
│   │   │   ├── mod.rs               # Connection pool + migrations
│   │   │   ├── migrations/          # SQL migrations (10 files)
│   │   │   └── models.rs            # All structs + CRUD functions
│   │   ├── middleware/
│   │   │   ├── auth.rs              # User API Key auth + model permission
│   │   │   ├── admin_auth.rs        # Admin JWT auth + role check
│   │   │   ├── user_auth.rs         # User JWT auth (no admin role required)
│   │   │   └── rate_limit.rs        # Global QPS/concurrency limits
│   │   ├── router/
│   │   │   ├── proxy_routes.rs      # Proxy requests + user self-query
│   │   │   └── admin_routes.rs      # Admin CRUD + stats + user endpoints
│   │   ├── service/
│   │   │   ├── proxy.rs             # HTTP proxy (streaming/non-streaming)
│   │   │   ├── openai.rs            # OpenAI format adapter
│   │   │   ├── anthropic.rs         # Anthropic format adapter
│   │   │   ├── quota.rs             # Quota check & deduction
│   │   │   ├── health.rs            # Channel health checks
│   │   │   └── log.rs               # Request logging
│   │   └── utils/
│   │       ├── jwt.rs               # JWT token generation & verification
│   │       └── token_counter.rs     # Token estimation
│   └── config/config.yaml           # Configuration file
├── web/
│   └── src/
│       ├── views/
│       │   ├── home/                # Dashboard (stats cards + charts)
│       │   ├── user/key/            # Key Management (all users)
│       │   ├── routing/             # Request Routing
│       │   │   ├── app-profile/     #   App Presets (admin)
│       │   │   └── traffic-plan/    #   Traffic Plans (admin)
│       │   ├── upstream/            # Upstream Models
│       │   │   ├── channel/         #   Channel Management (admin)
│       │   │   └── model/           #   Model Management (admin)
│       │   ├── control/             # Traffic Control
│       │   │   ├── quota/           #   Quota Management (admin)
│       │   │   ├── ratelimit/       #   Rate Limit Management (admin)
│       │   │   └── user/            #   User Management (admin)
│       │   ├── data/                # Data Statistics
│       │   │   ├── log/             #   Request Logs (all, user sees own)
│       │   │   └── stats/           #   Usage Stats (all, user sees own)
│       │   └── settings/            # System Settings (admin)
│       ├── service/api/             # API request wrappers
│       ├── typings/api/             # TypeScript type definitions
│       ├── router/elegant/          # Auto-generated routes
│       └── layouts/                 # Layout components (with version footer)
├── .github/workflows/
│   ├── docker.yml                   # Docker image auto-build & push
│   └── release.yml                  # Release auto-publish
├── Dockerfile                       # Multi-stage single-image build
├── docker-compose.yml               # Docker Compose orchestration
├── nginx.conf                       # Nginx reverse proxy config
├── start.sh                         # Container startup script
└── deploy.sh                        # Native one-click deploy script
```

---

## Permission Model

| Role | Description | Access |
|------|-------------|--------|
| **admin** | Administrator | Full access: channels, models, users, quotas, rate limits, settings |
| **user** | Regular user | Dashboard (personal data), Key Management (own API Keys), Data Stats (own logs & stats) |

Backend `admin_auth_middleware` enforces `role == "admin"`. Frontend routes use `roles: ['admin']` metadata for menu visibility.

**API Key Permissions:**
- Each user can create multiple API Keys
- Each Key can have an independent list of accessible models (`enabled_models`)
- Proxy layer enforces model access checks per Key

---

## Deployment Options

| Method | Use Case | Command |
|--------|----------|---------|
| **Docker Hub Image** | Simplest deployment, no compilation needed | `docker compose up -d` |
| **Docker Local Build** | Custom code modifications | `docker compose up -d --build` |
| **Native Deploy** | No Docker environment | `./deploy.sh` |
| **Dev Environment** | Local development | `cargo run` + `pnpm dev` |

### Version Update Detection

The sidebar footer displays the current version number and auto-checks GitHub Releases for updates every hour at 00:00. When a new version is available, it shows in red highlight and clicking navigates to the GitHub project page.

### GitHub Releases

Pushing a new version tag automatically triggers GitHub Actions to create a Release and upload a one-click deployment package (containing `docker-compose.yml`, `.env.example`, `config.yaml`).

---

## Tech Stack

### Backend

| Component | Technology |
|-----------|-----------|
| Language | Rust 2024 Edition |
| Web Framework | [Axum](https://github.com/tokio-rs/axum) 0.8 |
| Database | SQLite + [SQLx](https://github.com/launchbadge/sqlx) 0.8 |
| Async Runtime | [Tokio](https://tokio.rs/) 1 |
| Serialization | [Serde](https://serde.rs/) |
| Rate Limiting | [Governor](https://github.com/antifuchs/governor) 0.6 |
| JWT | [jsonwebtoken](https://github.com/Keats/jsonwebtoken) 9 |
| Logging | [tracing](https://github.com/tokio-rs/tracing) 0.1 + [tracing-appender](https://github.com/tokio-rs/tracing) (daily rotation) |
| HTTP Client | [reqwest](https://github.com/seanmonstar/reqwest) 0.12 |

### Frontend

| Component | Technology |
|-----------|-----------|
| Framework | Vue 3 + TypeScript |
| UI Library | [Naive UI](https://www.naiveui.com/) |
| Charts | [ECharts](https://echarts.apache.org/) 6 |
| Template | [SoybeanAdmin](https://github.com/soybeanjs/soybean-admin) |
| Request Library | [Alova](https://alova.js.org/) |
| Router | [elegant-router](https://github.com/soybeanjs/elegant-router) |

### Deployment Architecture

```
User Request → Nginx (Port 80)
              ├── /admin/* → Backend API (Port 8000)
              ├── /v1/*    → Backend Proxy (Port 8000)
              └── /*       → Vue SPA Static Files
```

---

## Main Business Flow

```
┌──────────┐     API Key      ┌──────────────┐     Channel      ┌──────────────┐
│   User    │ ──────────────→ │  CoRide-API   │ ──────────────→ │ Upstream AI    │
│  (API Key) │ ←────────────── │   Proxy        │ ←────────────── │ OpenAI/        │
└──────────┘   JSON Response  └──────────────┘   Raw Response   │ Anthropic/etc  │
                                                                     └──────────────┘
```

1. **User Request**: Sends request via OpenAI-compatible endpoint (`/v1/chat/completions`) with API Key
2. **Authentication**: `auth.rs` validates API Key, checks user status and rate limits
3. **Model Permission**: Checks if requested model is within user's `enabled_models` scope
4. **Quota Check**: Checks channel quota first, then user quota
5. **Channel Selection**: Finds matching channels by model name (weighted round-robin)
6. **App Spoofing**: `resolve_app_profile_for_channel` selects spoof config by channel plan → global plan → legacy app_profile_id
7. **Proxy Forward**: Forwards to upstream service (streaming SSE passthrough or non-streaming)
8. **Response Handling**: Parses Token usage, deducts quota, logs request
9. **Retry**: Auto-retries on 5xx errors by switching to next channel

---

## License

This project is licensed under the [Apache-2.0 License](LICENSE).

---

## Acknowledgments

- **[Axum](https://github.com/tokio-rs/axum)** — Rust Web framework
- **[Tokio](https://tokio.rs/)** — Rust async runtime
- **[SQLx](https://github.com/launchbadge/sqlx)** — Rust SQL toolkit
- **[Vue 3](https://vuejs.org/)** — Frontend framework
- **[Naive UI](https://www.naiveui.com/)** — Vue 3 component library
- **[ECharts](https://echarts.apache.org/)** — Data visualization
- **[SoybeanAdmin](https://github.com/soybeanjs/soybean-admin)** — Admin template
- **[Governor](https://github.com/antifuchs/governor)** — Rate limiting
- **[jsonwebtoken](https://github.com/Keats/jsonwebtoken)** — JWT tokens
- **[reqwest](https://github.com/seanmonstar/reqwest)** — HTTP client
- **[tracing](https://github.com/tokio-rs/tracing)** — Structured logging
- **[bcrypt](https://github.com/Keats/rust-bcrypt)** — Password hashing

---

## Contact

- **Email:** [cyfor@foxmail.com](mailto:cyfor@foxmail.com)
- **Feedback:** Issues and PRs are welcome
