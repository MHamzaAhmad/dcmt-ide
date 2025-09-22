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
# Default to 1000:1000 to align with typical host users and the launcher config
ARG USER_UID=1000
ARG USER_GID=1000

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    nginx \
    supervisor \
    ca-certificates \
    wget \
    python3 \
    python3-pip \
    python3-venv \
    && rm -rf /var/lib/apt/lists/*

# Install LiteLLM and Clerk Python SDK
RUN python3 -m venv /app/litellm-venv && \
    /app/litellm-venv/bin/pip install --upgrade pip && \
    /app/litellm-venv/bin/pip install 'litellm[proxy]'

# Document effective runtime IDs (names not required; we run with numeric IDs)
ENV APP_UID=${USER_UID} APP_GID=${USER_GID}

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
    && chown -R ${USER_UID}:${USER_GID} /app \
    && chown -R ${USER_UID}:${USER_GID} /var/log/supervisor \
    && chown -R ${USER_UID}:${USER_GID} /var/lib/nginx \
    && chown -R ${USER_UID}:${USER_GID} /run/nginx \
    && chmod -R 0775 /var/log/supervisor /var/lib/nginx /run/nginx

# Ensure logs directory is writable
RUN chmod 0775 /app/logs

# Copy built frontend
COPY --from=frontend-builder --chown=${USER_UID}:${USER_GID} /app/build /app/frontend

# Copy built backend
COPY --from=backend-builder --chown=${USER_UID}:${USER_GID} /app/backend/target/release/dcmt-backend /app/backend/dcmt-backend

# Copy configuration files
COPY --chown=${USER_UID}:${USER_GID} docker/nginx-internal.conf /etc/nginx/nginx.conf
COPY --chown=${USER_UID}:${USER_GID} docker/supervisord.conf /etc/supervisord.conf
COPY --chown=${USER_UID}:${USER_GID} docker/docker-entrypoint.sh /app/docker-entrypoint.sh

# Copy LiteLLM configuration
COPY --chown=${USER_UID}:${USER_GID} litellm/config.yaml /app/litellm/config.yaml

# Copy prompts directory
COPY --chown=${USER_UID}:${USER_GID} prompts/ /app/prompts/

# Make scripts executable
RUN chmod +x /app/docker-entrypoint.sh

## Create workspace directory owned by numeric runtime user; runtime will be rootless
RUN mkdir -p /workspace && \
    chown -R ${USER_UID}:${USER_GID} /workspace && \
    chmod 0775 /workspace

# Declare workspace as a volume (matches orchestrator mount target)
VOLUME ["/workspace"]

# Expose unprivileged port for internal HTTP traffic (rootless)
EXPOSE 3000

# Set environment variables
ENV DCMT_HOST=0.0.0.0
ENV DCMT_PORT=3001
ENV DCMT_WORKSPACE_PATH=/workspace
ENV LITELLM_BASE_URL=http://127.0.0.1:4000

## Rootless runtime: switch to numeric user
USER ${USER_UID}:${USER_GID}

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3000/ || exit 1

ENTRYPOINT ["/app/docker-entrypoint.sh"]
CMD ["/usr/bin/supervisord", "-n", "-c", "/etc/supervisord.conf"]