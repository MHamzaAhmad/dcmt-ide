# Multi-stage Docker build for dcmt-editor
FROM node:22-alpine AS frontend-builder

# Build arguments for environment variables
ARG VITE_CLERK_PUBLISHABLE_KEY
ARG VITE_CLERK_SIGN_IN_URL

# Set as environment variables for the build
ENV VITE_CLERK_PUBLISHABLE_KEY=$VITE_CLERK_PUBLISHABLE_KEY
ENV VITE_CLERK_SIGN_IN_URL=$VITE_CLERK_SIGN_IN_URL

WORKDIR /app

# Copy package files
COPY package.json pnpm-lock.yaml ./

# Install pnpm and dependencies
RUN npm install -g pnpm && pnpm install --frozen-lockfile

# Copy source code
COPY . .

# Build the frontend with environment variables
RUN pnpm run build

# Backend build stage
FROM rust:1.89-alpine AS backend-builder

# Install build dependencies including static OpenSSL libraries
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

WORKDIR /app

# Copy Cargo files
COPY backend/Cargo.toml backend/Cargo.lock backend/

# Copy prompts directory that the backend needs at build time
COPY prompts/ prompts/

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

# Allow configuring container user to match host UID/GID for volume permissions
ARG USER_UID=1001
ARG USER_GID=1001

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    nginx \
    supervisor \
    ca-certificates \
    wget \
    gosu \
    python3 \
    python3-pip \
    python3-venv \
    && rm -rf /var/lib/apt/lists/*

# Install LiteLLM and Clerk Python SDK
RUN python3 -m venv /app/litellm-venv && \
    /app/litellm-venv/bin/pip install --upgrade pip && \
    /app/litellm-venv/bin/pip install 'litellm[proxy]'

# Create app user with specific UID/GID for better volume compatibility
RUN groupadd -g ${USER_GID} appgroup && \
    useradd -u ${USER_UID} -g appgroup -m -s /bin/bash appuser

# Document effective runtime IDs
ENV APP_USER=appuser APP_GROUP=appgroup APP_UID=${USER_UID} APP_GID=${USER_GID}

# Create necessary directories
RUN mkdir -p /app/frontend \
    /app/backend \
    /app/logs \
    /var/log/supervisor \
    /run/nginx \
    /var/lib/nginx/body \
    /var/lib/nginx/proxy \
    /var/lib/nginx/fastcgi \
    /var/lib/nginx/uwsgi \
    /var/lib/nginx/scgi \
    && chown -R appuser:appgroup /app \
    && chown -R appuser:appgroup /var/log/supervisor \
    && chown -R appuser:appgroup /var/lib/nginx \
    && chown -R appuser:appgroup /run/nginx \
    && chmod -R 0777 /var/log/supervisor /var/lib/nginx /run/nginx

# Ensure logs directory is writable
RUN chmod 0777 /app/logs

# Copy built frontend
COPY --from=frontend-builder --chown=appuser:appgroup /app/build /app/frontend

# Copy built backend
COPY --from=backend-builder --chown=appuser:appgroup /app/backend/target/release/dcmt-backend /app/backend/dcmt-backend

# Copy configuration files
COPY --chown=appuser:appgroup docker/nginx-internal.conf /etc/nginx/nginx.conf
COPY --chown=appuser:appgroup docker/supervisord.conf /etc/supervisord.conf
COPY --chown=appuser:appgroup docker/docker-entrypoint.sh /app/docker-entrypoint.sh

# Copy LiteLLM configuration
COPY --chown=appuser:appgroup litellm/config.yaml /app/litellm/config.yaml

# Copy prompts directory
COPY --chown=appuser:appgroup prompts/ /app/prompts/

# Make scripts executable
RUN chmod +x /app/docker-entrypoint.sh

# Create workspace directory with permissive permissions to support arbitrary runtime UID/GID
# This avoids write issues when the container is run with a different user ID
RUN mkdir -p /app/workspace && \
    chown -R appuser:appgroup /app/workspace && \
    chmod 0777 /app/workspace

# Declare workspace as a volume for proper handling
VOLUME ["/app/workspace"]

# Expose unprivileged port for internal HTTP traffic (rootless)
EXPOSE 3000

# Set environment variables
ENV DCMT_HOST=0.0.0.0
ENV DCMT_PORT=3001
ENV DCMT_WORKSPACE_PATH=/app/workspace
ENV LITELLM_BASE_URL=http://127.0.0.1:4000

# NOTE: We intentionally stay as root here so the entrypoint can fix ownership
# of volume-mounted workspace directories, then drop privileges with gosu.

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3000/ || exit 1

ENTRYPOINT ["/app/docker-entrypoint.sh"]
CMD ["/usr/bin/supervisord", "-n", "-c", "/etc/supervisord.conf"]