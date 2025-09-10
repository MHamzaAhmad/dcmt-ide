# Multi-stage Docker build for dcmt-editor with SSL support
FROM node:20-alpine AS frontend-builder

WORKDIR /app

# Copy package files
COPY package.json pnpm-lock.yaml ./

# Install pnpm and dependencies
RUN npm install -g pnpm && pnpm install --frozen-lockfile

# Copy source code
COPY . .

# Build the frontend
RUN pnpm run build

# Backend build stage
FROM rust:1.75-alpine AS backend-builder

# Install build dependencies
RUN apk add --no-cache musl-dev pkgconfig openssl-dev

WORKDIR /app

# Copy Cargo files
COPY backend/Cargo.toml backend/Cargo.lock backend/

# Create dummy main.rs for dependency caching
RUN mkdir -p backend/src/cmd && echo "fn main() {}" > backend/src/cmd/main.rs

# Build dependencies
WORKDIR /app/backend
RUN cargo build --release

# Copy actual source code
COPY backend/src ./src

# Build the actual application
RUN touch src/cmd/main.rs && cargo build --release

# Production stage
FROM texlive/texlive:latest

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    nginx \
    supervisor \
    openssl \
    ca-certificates \
    wget \
    sudo \
    python3 \
    python3-pip \
    python3-venv \
    && rm -rf /var/lib/apt/lists/*

# Install LiteLLM
RUN python3 -m venv /app/litellm-venv && \
    /app/litellm-venv/bin/pip install --upgrade pip && \
    /app/litellm-venv/bin/pip install 'litellm[proxy]'

# Create app user
RUN groupadd -g 1001 appgroup && \
    useradd -u 1001 -g appgroup -m -s /bin/bash appuser

# Create necessary directories
RUN mkdir -p /app/frontend \
    /app/backend \
    /app/ssl \
    /app/logs \
    /var/log/supervisor \
    /run/nginx \
    && chown -R appuser:appgroup /app \
    && chown -R appuser:appgroup /var/log/supervisor

# Copy built frontend
COPY --from=frontend-builder --chown=appuser:appgroup /app/build /app/frontend

# Copy built backend
COPY --from=backend-builder --chown=appuser:appgroup /app/backend/target/release/dcmt-backend /app/backend/dcmt-backend

# Copy configuration files
COPY --chown=appuser:appgroup docker/nginx-ssl.conf /etc/nginx/nginx.conf
COPY --chown=appuser:appgroup docker/supervisord.conf /etc/supervisord.conf
COPY --chown=appuser:appgroup docker/docker-entrypoint.sh /app/docker-entrypoint.sh
COPY --chown=appuser:appgroup docker/generate-ssl.sh /app/generate-ssl.sh

# Copy LiteLLM configuration
COPY --chown=appuser:appgroup litellm/config.yaml /app/litellm/config.yaml

# Make scripts executable
RUN chmod +x /app/docker-entrypoint.sh /app/generate-ssl.sh

# Create workspace directory with proper permissions
RUN mkdir -p /app/workspace && chown -R appuser:appgroup /app/workspace

# Set proper permissions for the appuser to access mounted volumes
RUN echo "appuser ALL=(ALL) NOPASSWD: /bin/chown, /bin/chmod" >> /etc/sudoers || true

# Expose ports
EXPOSE 80 443

# Set environment variables
ENV DCMT_HOST=0.0.0.0
ENV DCMT_PORT=3001
ENV DCMT_WORKSPACE_PATH=/app/workspace
ENV LITELLM_BASE_URL=http://127.0.0.1:4000

# Switch to app user
USER appuser

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider https://localhost:443/ || exit 1

ENTRYPOINT ["/app/docker-entrypoint.sh"]
CMD ["/usr/bin/supervisord", "-c", "/etc/supervisord.conf"]