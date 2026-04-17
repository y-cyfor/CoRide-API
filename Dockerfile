# Stage 1: Build backend
FROM rust:1.80-slim AS backend-builder

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

EXPOSE 80

# Startup script: launch backend and nginx together
RUN printf '#!/bin/bash\n\
/app/coride-api &\n\
exec nginx -g "daemon off;"\n' /app/start.sh && chmod +x /app/start.sh

CMD ["/app/start.sh"]
