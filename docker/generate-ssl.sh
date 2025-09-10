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

# Use certbot to get the certificate
certbot certonly \
    --standalone \
    --non-interactive \
    --agree-tos \
    $EMAIL_ARG \
    $STAGING_FLAG \
    --domains "$DOMAIN" \
    --cert-path "$CERT_FILE" \
    --key-path "$KEY_FILE" \
    --fullchain-path "$CERT_FILE" \
    --work-dir /tmp/letsencrypt \
    --logs-dir /app/logs

# Check if certificates were created
if [ -f "$CERT_FILE" ] && [ -f "$KEY_FILE" ]; then
    echo "✅ Let's Encrypt certificate obtained successfully!"
    
    # Set proper permissions
    chown appuser:appgroup "$CERT_FILE" "$KEY_FILE"
    chmod 644 "$CERT_FILE"
    chmod 600 "$KEY_FILE"
    
    echo "📊 Certificate details:"
    echo "  - Certificate: $CERT_FILE"
    echo "  - Private key: $KEY_FILE"
    echo "  - Domain: $DOMAIN"
    
    # Verify the certificate
    echo "🔍 Certificate verification:"
    openssl x509 -in "$CERT_FILE" -text -noout | grep -E "(Subject:|DNS:)" || true
else
    echo "❌ Failed to obtain Let's Encrypt certificate"
    exit 1
fi