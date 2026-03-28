#!/bin/bash
# Run this on the VPS after deploying to set up SSL with certbot.
# Usage: ./setup-ssl.sh yourdomain.com admin@yourdomain.com
#
# Prerequisites:
#   - DNS A records pointing to this server for:
#     exchange.yourdomain.com, timer.yourdomain.com, admin.yourdomain.com
#   - Container running with port 80 exposed

set -euo pipefail

DOMAIN="${1:?Usage: $0 <domain> <email>}"
EMAIL="${2:?Usage: $0 <domain> <email>}"

echo "==> Setting up SSL for *.${DOMAIN}"

# Update nginx.conf with actual domain
sed -i "s/YOURDOMAIN\.com/${DOMAIN}/g" /etc/nginx/nginx.conf
nginx -s reload

# Get certificates for all subdomains
certbot --nginx --non-interactive --agree-tos \
  -m "${EMAIL}" \
  -d "exchange.${DOMAIN}" \
  -d "timer.${DOMAIN}" \
  -d "admin.${DOMAIN}"

echo "==> SSL setup complete"
echo "==> Set up auto-renewal: certbot renew --dry-run"
