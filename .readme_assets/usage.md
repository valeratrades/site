# Usage & Deployment

Instructions for deploying the site with HTTPS using nginx or Caddy, and configuring DNS.

## Table of Contents
- [Option 1: Nginx with Let's Encrypt (Recommended for existing nginx setups)](#option-1-nginx-with-lets-encrypt)
- [Option 2: Caddy (Easiest - automatic HTTPS)](#option-2-caddy)
- [DNS Configuration (Squarespace/Any Provider)](#dns-configuration)
- [Verification & Troubleshooting](#verification--troubleshooting)

---

## Option 1: Nginx with Let's Encrypt

Use this if nginx is already installed or you prefer more granular control.

### Step 1: Install Nginx and Certbot
```sh
sudo apt update
sudo apt install -y nginx certbot python3-certbot-nginx
```

### Step 2: Create Nginx Configuration
```sh
# Create site configuration
sudo tee /etc/nginx/sites-available/valeratrades.com > /dev/null <<'EOF'
server {
    listen 80;
    listen [::]:80;
    server_name valeratrades.com www.valeratrades.com;

    location / {
        proxy_pass http://127.0.0.1:61156;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
EOF

# Enable the site
sudo ln -sf /etc/nginx/sites-available/valeratrades.com /etc/nginx/sites-enabled/valeratrades.com

# Test configuration
sudo nginx -t

# Reload nginx
sudo systemctl reload nginx
```

### Step 3: Obtain SSL Certificate
```sh
# Get certificate and auto-configure HTTPS
sudo certbot --nginx -d valeratrades.com -d www.valeratrades.com --non-interactive --agree-tos --register-unsafely-without-email --redirect

# Or with email for renewal notifications:
sudo certbot --nginx -d valeratrades.com -d www.valeratrades.com --non-interactive --agree-tos --email your-email@example.com --redirect
```

### Step 4: Verify Auto-Renewal
```sh
# Test renewal process (dry run)
sudo certbot renew --dry-run

# Check certbot timer is enabled
sudo systemctl status certbot.timer
```

### Managing Nginx

```sh
# Restart nginx
sudo systemctl restart nginx

# Reload nginx (no downtime)
sudo systemctl reload nginx

# View nginx logs
sudo tail -f /var/log/nginx/access.log
sudo tail -f /var/log/nginx/error.log

# Test configuration
sudo nginx -t

# View site configuration
cat /etc/nginx/sites-enabled/valeratrades.com
```

---

## Option 2: Caddy

Caddy automatically obtains and renews SSL certificates with zero configuration.

### Step 1: Install Caddy
```sh
# Add Caddy repository
sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https curl
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list

# Install Caddy
sudo apt update
sudo apt install caddy
```

### Step 2: Configure Caddy
```sh
# Create Caddyfile
sudo tee /etc/caddy/Caddyfile > /dev/null <<'EOF'
valeratrades.com {
    reverse_proxy localhost:61156
}

www.valeratrades.com {
    redir https://valeratrades.com{uri} permanent
}
EOF

# Reload Caddy
sudo systemctl reload caddy
```

That's it! Caddy automatically:
- Obtains SSL certificates from Let's Encrypt
- Renews certificates before expiration
- Redirects HTTP to HTTPS

### Managing Caddy

```sh
# Restart Caddy
sudo systemctl restart caddy

# Reload configuration (no downtime)
sudo systemctl reload caddy

# View Caddy logs
sudo journalctl -u caddy -f

# Check Caddy status
sudo systemctl status caddy

# Validate Caddyfile syntax
caddy validate --config /etc/caddy/Caddyfile
```

---

## DNS Configuration

Configure DNS to point your domain to your server. This works for any DNS provider (Squarespace, Cloudflare, Route53, etc.).

### For Squarespace DNS

1. Log into Squarespace
2. Go to **Settings** → **Domains** → **valeratrades.com**
3. Click **DNS Settings**
4. Add/Update the following records:

| Type  | Host | Value              | TTL  |
|-------|------|--------------------|------|
| A     | @    | `YOUR_SERVER_IP`   | 3600 |
| A     | www  | `YOUR_SERVER_IP`   | 3600 |

**Important**:
- Replace `YOUR_SERVER_IP` with your actual server IP address
- If you're using Squarespace for email, preserve any MX records
- DNS changes can take 5 minutes to 48 hours to propagate (usually ~1 hour)

### For Other DNS Providers

**Cloudflare:**
1. Dashboard → Domain → DNS
2. Add A records for `@` and `www` pointing to your server IP
3. Set proxy status to "DNS only" (gray cloud) initially for testing
4. After verification, enable proxy (orange cloud) for DDoS protection and CDN

**Route53 (AWS):**
```sh
# Get your hosted zone ID
aws route53 list-hosted-zones

# Create A records
aws route53 change-resource-record-sets --hosted-zone-id YOUR_ZONE_ID --change-batch '{
  "Changes": [
    {
      "Action": "UPSERT",
      "ResourceRecordSet": {
        "Name": "valeratrades.com",
        "Type": "A",
        "TTL": 300,
        "ResourceRecords": [{"Value": "YOUR_SERVER_IP"}]
      }
    },
    {
      "Action": "UPSERT",
      "ResourceRecordSet": {
        "Name": "www.valeratrades.com",
        "Type": "A",
        "TTL": 300,
        "ResourceRecords": [{"Value": "YOUR_SERVER_IP"}]
      }
    }
  ]
}'
```

**Generic DNS (GoDaddy, Namecheap, etc.):**
1. Log into your DNS provider
2. Navigate to DNS Management / DNS Records
3. Add two A records:
   - Host: `@`, Points to: `YOUR_SERVER_IP`
   - Host: `www`, Points to: `YOUR_SERVER_IP`

### Finding Your Server IP

```sh
# On your server, run:
curl ifconfig.me

# Or:
ip addr show | grep 'inet ' | grep -v 127.0.0.1
```

### Verify DNS Propagation

```sh
# Check if DNS is resolving
dig valeratrades.com
dig www.valeratrades.com

# Or use online tools:
# - https://dnschecker.org
# - https://www.whatsmydns.net
```

---

## Firewall Configuration

Ensure your firewall allows HTTP and HTTPS traffic:

```sh
# Using ufw (Ubuntu/Debian)
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw enable
sudo ufw status

# Using firewalld (RHEL/CentOS)
sudo firewall-cmd --permanent --add-service=http
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --reload

# Using iptables (manual)
sudo iptables -A INPUT -p tcp --dport 80 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 443 -j ACCEPT
sudo iptables-save
```

---

## Verification & Troubleshooting

### Test HTTPS Connection
```sh
# Test from command line
curl -I https://valeratrades.com

# Expected output should include:
# HTTP/2 200 (or 301/302 for redirects)
# Connection should NOT show certificate errors
```

### Test HTTP to HTTPS Redirect
```sh
curl -I http://valeratrades.com

# Expected: HTTP/1.1 301 Moved Permanently
# Location: https://valeratrades.com/
```

### Check Certificate Details
```sh
# View certificate info
echo | openssl s_client -connect valeratrades.com:443 -servername valeratrades.com 2>/dev/null | openssl x509 -noout -dates -subject

# Or use online tools:
# https://www.ssllabs.com/ssltest/
```

### Common Issues

#### 1. Certificate Not Found / Invalid
```sh
# For nginx:
sudo certbot certificates
sudo certbot renew --force-renewal

# For Caddy:
# Caddy manages this automatically, but check logs:
sudo journalctl -u caddy -f
```

#### 2. Site Not Accessible
```sh
# Check if your app is running
curl http://127.0.0.1:61156

# Check reverse proxy logs
# For nginx:
sudo tail -f /var/log/nginx/error.log

# For Caddy:
sudo journalctl -u caddy -f

# Check app logs
sudo journalctl -u valeratrades -f
```

#### 3. DNS Not Resolving
```sh
# Check DNS records
dig valeratrades.com
nslookup valeratrades.com

# Try different DNS servers
dig @8.8.8.8 valeratrades.com  # Google DNS
dig @1.1.1.1 valeratrades.com  # Cloudflare DNS

# Wait for propagation (can take up to 48 hours)
```

#### 4. 502 Bad Gateway
```sh
# Your reverse proxy can't reach the app
# Verify app is running:
sudo systemctl status valeratrades

# Restart app:
sudo systemctl restart valeratrades

# Check app logs:
sudo journalctl -u valeratrades -n 50
```

#### 5. Renewal Issues
```sh
# For nginx + certbot:
# Check renewal timer
sudo systemctl status certbot.timer

# Manually renew
sudo certbot renew

# For Caddy:
# Caddy handles renewal automatically
# Check logs if issues occur:
sudo journalctl -u caddy | grep -i renew
```

---

## Updating the Site

### Standard Update Process
```sh
# 1. SSH into server
ssh cloudzy_ubuntu  # or your server alias

# 2. Navigate to site directory
cd ~/s/site

# 3. Pull latest changes
git pull

# 4. Rebuild
nix build --rebuild

# 5. Restart the service
sudo systemctl restart valeratrades

# 6. Verify it's working
curl -I https://valeratrades.com
```

### Rolling Back
```sh
# If something breaks, revert to previous commit
git log --oneline  # Find the commit hash
git checkout <previous-commit-hash>
nix build --rebuild
sudo systemctl restart valeratrades
```

---

## Monitoring

### View Application Logs
```sh
# Real-time logs
sudo journalctl -u valeratrades -f

# Last 100 lines
sudo journalctl -u valeratrades -n 100

# Logs from today
sudo journalctl -u valeratrades --since today

# Logs with errors only
sudo journalctl -u valeratrades -p err
```

### Check Service Status
```sh
# Application status
sudo systemctl status valeratrades

# Nginx/Caddy status
sudo systemctl status nginx
# OR
sudo systemctl status caddy

# Check if ports are listening
sudo ss -tlnp | grep -E '(80|443|61156)'
```

### Resource Usage
```sh
# Check CPU and memory
top
# Or:
htop

# Specific to your app
ps aux | grep site
```
