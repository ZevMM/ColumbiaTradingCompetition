# ── Stage 1: Build matching engine ────────────────────────────────
FROM rust:latest AS engine-builder
WORKDIR /build
COPY matching-engine/Cargo.toml matching-engine/Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release
COPY matching-engine/ .
RUN touch src/main.rs && cargo build --release

# ── Stage 2: Build exchange client ───────────────────────────────
FROM node:20-slim AS client-builder
WORKDIR /build
COPY exchange-client/package.json exchange-client/package-lock.json ./
RUN npm ci
COPY exchange-client/ .
# Override base path for self-hosted deployment (served at /)
RUN sed -i 's|base:.*|base: "/",|' vite.config.js && npm run build

# ── Stage 3: Build timer ─────────────────────────────────────────
FROM node:20-slim AS timer-builder
WORKDIR /build
COPY timer/package.json timer/package-lock.json ./
RUN npm ci
COPY timer/ .
RUN npm run build

# ── Stage 4: Final image ─────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    nginx \
    python3 \
    python3-pip \
    python3-venv \
    supervisor \
    curl \
    apache2-utils \
    certbot \
    python3-certbot-nginx \
    && rm -rf /var/lib/apt/lists/*

# Set up Python venv for admin server + bots
RUN python3 -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

# Install Python dependencies
COPY admin/requirements.txt /tmp/admin-requirements.txt
RUN pip install --no-cache-dir -r /tmp/admin-requirements.txt websockets numpy

# ── Copy built artifacts ─────────────────────────────────────────
# Matching engine binary
COPY --from=engine-builder /build/target/release/main /app/matching-engine/main
COPY matching-engine/config.json /app/matching-engine/config.json

# Exchange client static build
COPY --from=client-builder /build/dist /app/exchange-client/dist

# Timer static build
COPY --from=timer-builder /build/dist /app/timer/dist

# Python bots
COPY python_bots/ /app/python_bots/

# Admin server
COPY admin/ /app/admin/

# ── Config files ─────────────────────────────────────────────────
COPY deploy/supervisord.conf /etc/supervisor/conf.d/supervisord.conf

# Use local nginx config by default; override with NGINX_CONF build arg for prod
ARG NGINX_CONF=deploy/nginx.local.conf
COPY ${NGINX_CONF} /etc/nginx/nginx.conf

# Create default htpasswd (change this!)
RUN htpasswd -cb /etc/nginx/.htpasswd admin r85.jHpX7tYLmuUo0hazHjWz*

# ── Volumes for runtime data ─────────────────────────────────────
VOLUME ["/app/matching-engine/config.json", "/app/python_bots/data"]

EXPOSE 80 443

CMD ["supervisord", "-c", "/etc/supervisor/conf.d/supervisord.conf"]
