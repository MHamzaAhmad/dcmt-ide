#!/bin/sh
set -e

echo "🔧 Generating self-signed SSL certificate..."

# Create SSL directory if it doesn't exist
mkdir -p /app/ssl

# Certificate configuration
CERT_FILE="/app/ssl/cert.pem"
KEY_FILE="/app/ssl/key.pem"
DAYS=365
COUNTRY="US"
STATE="CA"
CITY="San Francisco"
ORG="DCMT Editor"
OU="Development"
CN="${DOMAIN:-localhost}"

# Generate private key
echo "🔑 Generating private key..."
openssl genrsa -out "$KEY_FILE" 2048

# Generate certificate signing request
echo "📝 Generating certificate signing request..."
openssl req -new -key "$KEY_FILE" -out /tmp/cert.csr -subj "/C=$COUNTRY/ST=$STATE/L=$CITY/O=$ORG/OU=$OU/CN=$CN"

# Create certificate extensions file for SAN
cat > /tmp/cert.ext << EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = *.localhost
DNS.3 = $CN
IP.1 = 127.0.0.1
IP.2 = ::1
EOF

# If domain is specified, add it to SAN
if [ ! -z "$DOMAIN" ]; then
    echo "DNS.4 = $DOMAIN" >> /tmp/cert.ext
    echo "DNS.5 = *.$DOMAIN" >> /tmp/cert.ext
fi

# Generate self-signed certificate
echo "📜 Generating self-signed certificate..."
openssl x509 -req -in /tmp/cert.csr -signkey "$KEY_FILE" -out "$CERT_FILE" -days $DAYS -extensions v3_req -extfile /tmp/cert.ext

# Set proper permissions
chown appuser:appgroup "$CERT_FILE" "$KEY_FILE"
chmod 644 "$CERT_FILE"
chmod 600 "$KEY_FILE"

# Clean up temporary files
rm -f /tmp/cert.csr /tmp/cert.ext

echo "✅ Self-signed SSL certificate generated successfully!"
echo "📊 Certificate details:"
echo "  - Certificate: $CERT_FILE"
echo "  - Private key: $KEY_FILE"
echo "  - Common Name: $CN"
echo "  - Valid for: $DAYS days"

# Verify the certificate
echo "🔍 Certificate verification:"
openssl x509 -in "$CERT_FILE" -text -noout | grep -E "(Subject:|DNS:|IP Address:)" || true

echo "⚠️  Note: This is a self-signed certificate. For production, use proper SSL certificates from a trusted CA."