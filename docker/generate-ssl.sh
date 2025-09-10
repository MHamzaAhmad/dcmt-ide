#!/bin/sh
set -e

echo "🔒 Setting up SSL certificate..."

# Create SSL directory if it doesn't exist
mkdir -p /app/ssl

# Certificate configuration
CERT_FILE="/app/ssl/cert.pem"
KEY_FILE="/app/ssl/key.pem"

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

echo "🔧 Obtaining Let's Encrypt certificate for $DOMAIN..."

# Create certbot directories with proper permissions
mkdir -p /app/letsencrypt/config /app/letsencrypt/work /app/letsencrypt/logs
chown -R appuser:appgroup /app/letsencrypt

# Use certbot to get the certificate
sudo certbot certonly \
    --standalone \
    --non-interactive \
    --agree-tos \
    $EMAIL_ARG \
    $STAGING_FLAG \
    --domains "$DOMAIN" \
    --config-dir /app/letsencrypt/config \
    --work-dir /app/letsencrypt/work \
    --logs-dir /app/letsencrypt/logs \
    --preferred-challenges http \
    --http-01-port 80

# Copy certificates to the expected location
echo "📁 Looking for certificates in: /app/letsencrypt/config/live/$DOMAIN"
if [ -d "/app/letsencrypt/config/live/$DOMAIN" ]; then
    echo "✓ Certificate directory found"
    echo "📋 Copying certificates to /app/ssl/"
    sudo cp /app/letsencrypt/config/live/$DOMAIN/fullchain.pem "$CERT_FILE"
    sudo cp /app/letsencrypt/config/live/$DOMAIN/privkey.pem "$KEY_FILE"
    sudo chown appuser:appgroup "$CERT_FILE" "$KEY_FILE"
    sudo chmod 644 "$CERT_FILE"
    sudo chmod 600 "$KEY_FILE"
else
    echo "❌ Certificate directory not found at: /app/letsencrypt/config/live/$DOMAIN"
    echo "📂 Checking available directories:"
    ls -la /app/letsencrypt/config/live/ || echo "No live directory found"
fi

# Check if certificates were created
echo "🔍 Checking for certificates at:"
echo "  - $CERT_FILE"
echo "  - $KEY_FILE"
ls -la /app/ssl/ || true

if [ -f "$CERT_FILE" ] && [ -f "$KEY_FILE" ]; then
    echo "✅ Let's Encrypt certificate obtained successfully!"
    
    echo "📊 Certificate details:"
    echo "  - Certificate: $CERT_FILE"
    echo "  - Private key: $KEY_FILE"
    echo "  - Domain: $DOMAIN"
    
    # Verify the certificate
    echo "🔍 Certificate verification:"
    openssl x509 -in "$CERT_FILE" -text -noout | grep -E "(Subject:|DNS:)" || true
else
    echo "❌ Failed to copy Let's Encrypt certificates to /app/ssl/"
    exit 1
fi