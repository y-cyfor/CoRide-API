# Stage 1: Build backend
FROM rust:1.88-slim AS backend-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --locked 2>/dev/null || true
RUN rm -rf src

COPY backend/ .
RUN cargo build --release

# Stage 2: Build frontend
FROM node:20-alpine AS frontend-builder

WORKDIR /app
COPY web/ .
RUN corepack enable && pnpm install --frozen-lockfile
RUN pnpm build

# Stage 3: Production image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 nginx \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Create data and log directories
RUN mkdir -p /app/data /app/log /app/config

# Copy backend binary
COPY --from=backend-builder /app/target/release/coride-api /app/coride-api

# Copy frontend static files
COPY --from=frontend-builder /app/dist /usr/share/nginx/html

# Copy nginx config
COPY nginx.conf /etc/nginx/sites-enabled/default

# Copy startup script
COPY start.sh /app/start.sh
RUN chmod +x /app/start.sh

EXPOSE 80

CMD ["/app/start.sh"]
