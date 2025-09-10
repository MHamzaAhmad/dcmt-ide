#!/bin/bash
set -e

echo "🔒 Setting up nginx with SSL certificate on host machine..."

# Check if running as root (required for nginx and certbot installation)
if [ "$EUID" -ne 0 ]; then
    echo "❌ This script must be run as root (use sudo)"
    echo "Usage: sudo ./docker/generate-ssl.sh"
    exit 1
fi

# Load environment variables from .env file if it exists
if [ -f ".env" ]; then
    echo "📋 Loading environment variables from .env file..."
    set -a
    source .env
    set +a
fi

if [ -z "$DOMAIN" ]; then
    echo "❌ Error: DOMAIN environment variable is required"
    echo "Please set DOMAIN in your .env file"
    exit 1
fi

echo "🌐 Domain: $DOMAIN"

# Check if we should use Let's Encrypt staging (for testing)
if [ "$LETSENCRYPT_STAGING" = "true" ]; then
    STAGING_FLAG="--staging"
    echo "⚠️  Using Let's Encrypt staging environment (for testing)"
else
    STAGING_FLAG=""
fi

# Email for Let's Encrypt (optional but recommended)
if [ -z "$LETSENCRYPT_EMAIL" ]; then
    EMAIL_ARG="--register-unsafely-without-email"
    echo "⚠️  No email provided for Let's Encrypt notifications"
else
    EMAIL_ARG="--email $LETSENCRYPT_EMAIL"
fi

# Update system packages
echo "📦 Updating system packages..."
apt-get update

# Install nginx and certbot
echo "🔧 Installing nginx and certbot..."
apt-get install -y nginx certbot python3-certbot-nginx

# Stop nginx temporarily for certificate generation
systemctl stop nginx || true

echo "🔧 Obtaining Let's Encrypt certificate for $DOMAIN..."

# Use certbot to get the certificate
certbot certonly \
    --standalone \
    --non-interactive \
    --agree-tos \
    $EMAIL_ARG \
    $STAGING_FLAG \
    --domains "$DOMAIN" \
    --preferred-challenges http \
    --http-01-port 80

# Create nginx configuration for DCMT Editor
echo "⚙️  Creating nginx configuration for $DOMAIN..."

# Get the current working directory (project root)
PROJECT_ROOT=$(pwd)

# First, add rate limiting zones to main nginx.conf
if ! grep -q "limit_req_zone" /etc/nginx/nginx.conf; then
    echo "Adding rate limiting configuration to nginx.conf..."
    sed -i '/http {/a\\n    # Rate limiting zones\n    limit_req_zone \$binary_remote_addr zone=api:10m rate=10r/s;\n    limit_req_zone \$binary_remote_addr zone=general:10m rate=30r/s;' /etc/nginx/nginx.conf
fi

cat > /etc/nginx/sites-available/dcmt-editor << EOF
# DCMT Editor nginx configuration
server {
    listen 80;
    server_name $DOMAIN;
    
    # Redirect all HTTP traffic to HTTPS
    return 301 https://\$host\$request_uri;
}

server {
    listen 443 ssl;
    http2 on;
    server_name $DOMAIN;

    # SSL certificates from Let's Encrypt
    ssl_certificate /etc/letsencrypt/live/$DOMAIN/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/$DOMAIN/privkey.pem;

    # SSL configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-RSA-AES128-SHA256:ECDHE-RSA-AES256-SHA384;
    ssl_prefer_server_ciphers off;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 10m;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;

    # Gzip compression
    gzip on;
    gzip_vary on;
    gzip_min_length 1024;
    gzip_types
        text/plain
        text/css
        text/xml
        text/javascript
        application/javascript
        application/xml+rss
        application/json
        application/xml
        image/svg+xml;

    # Proxy all requests to Docker container on port 80
    location / {
        limit_req zone=general burst=20 nodelay;
        
        proxy_pass http://127.0.0.1:80;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_set_header X-Forwarded-Host \$host;
        proxy_set_header X-Forwarded-Port \$server_port;
        
        proxy_cache_bypass \$http_upgrade;
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
        
        # For SSE and WebSocket support
        proxy_buffering off;
        chunked_transfer_encoding on;
    }

    # API routes with stricter rate limiting
    location /api/ {
        limit_req zone=api burst=10 nodelay;
        
        proxy_pass http://127.0.0.1:80;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_set_header X-Forwarded-Host \$host;
        proxy_set_header X-Forwarded-Port \$server_port;
        
        proxy_cache_bypass \$http_upgrade;
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }

    # Let's Encrypt challenge location
    location /.well-known/acme-challenge/ {
        root /var/www/html;
        allow all;
    }
}
EOF

# Enable the site
echo "🔗 Enabling nginx site configuration..."
ln -sf /etc/nginx/sites-available/dcmt-editor /etc/nginx/sites-enabled/

# Remove default nginx site if it exists
rm -f /etc/nginx/sites-enabled/default

# Test nginx configuration
echo "🧪 Testing nginx configuration..."
nginx -t

# Start and enable nginx
echo "🚀 Starting nginx..."
systemctl enable nginx
systemctl start nginx

# Setup automatic certificate renewal
echo "🔄 Setting up automatic certificate renewal..."
systemctl enable certbot.timer || echo "⚠️  Certbot timer not available on this system"

echo "✅ SSL setup completed successfully!"
echo ""
echo "📊 Setup Summary:"
echo "  - Domain: $DOMAIN"
echo "  - SSL Certificate: /etc/letsencrypt/live/$DOMAIN/fullchain.pem"
echo "  - SSL Private Key: /etc/letsencrypt/live/$DOMAIN/privkey.pem"
echo "  - Nginx Configuration: /etc/nginx/sites-available/dcmt-editor"
echo "  - Docker Container: Proxied on http://127.0.0.1:80"
echo ""
echo "🌐 Your DCMT Editor will be accessible at: https://$DOMAIN"
echo "📋 Make sure your Docker container is running on port 80"
echo ""
echo "🔧 To check nginx status: systemctl status nginx"
echo "🔧 To reload nginx: systemctl reload nginx"
echo "🔧 To check SSL certificate: certbot certificates"