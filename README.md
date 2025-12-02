# site
![Minimum Supported Rust Version](https://img.shields.io/badge/nightly-1.86+-ab6000.svg)
[<img alt="crates.io" src="https://img.shields.io/crates/v/site.svg?color=fc8d62&logo=rust" height="20" style=flat-square>](https://crates.io/crates/site)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs&style=flat-square" height="20">](https://docs.rs/site)
![Lines Of Code](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/site-loc.json)
<br>
[<img alt="ci errors" src="https://img.shields.io/github/actions/workflow/status/valeratrades/site/errors.yml?branch=master&style=for-the-badge&style=flat-square&label=errors&labelColor=420d09" height="20">](https://github.com/valeratrades/site/actions?query=branch%3Amaster) <!--NB: Won't find it if repo is private-->
[<img alt="ci warnings" src="https://img.shields.io/github/actions/workflow/status/valeratrades/site/warnings.yml?branch=master&style=for-the-badge&style=flat-square&label=warnings&labelColor=d16002" height="20">](https://github.com/valeratrades/site/actions?query=branch%3Amaster) <!--NB: Won't find it if repo is private-->

My sity site
<!-- markdownlint-disable -->
<details>
<summary>
<h3>Installation</h3>
</summary>

Full setup instructions to get the site running locally or on a server.

### Prerequisites

#### System Requirements
- NixOS or a system with Nix package manager
- Git

### Local Development Setup

#### 1. Clone the Repository
```bash
git clone <repository-url>
cd site
```

#### 2. Enter Nix Development Shell
```bash
nix develop
```

This will automatically:
- Install Rust nightly toolchain with wasm32-unknown-unknown target
- Install cargo-leptos (or update to latest version)
- Install frontend build tools (sassc, binaryen, tailwindcss)
- Build Tailwind CSS
- Set up git hooks and pre-commit checks

#### 3. Build and Run Locally
```bash
## Watch mode with hot reload (recommended for development)
lw  # alias for: cargo leptos watch --hot-reload

## Or manually:
cargo leptos watch --hot-reload
```

The site will be available at `http://127.0.0.1:61156`

#### 4. Production Build
```bash
## Build Tailwind CSS first
tailwindcss -i ./style/tailwind_in.css -o ./style/tailwind_out.css

## Build with Nix (recommended)
nix build

## The binary will be in: ./result/bin/site
./result/bin/site

## Or build with cargo-leptos
cargo leptos build --release
```

### Server Deployment Setup

#### 1. Install Dependencies on Server
```bash
## Update system
sudo apt update && sudo apt upgrade -y

## Install required packages
sudo apt install -y build-essential curl git

## Install Nix (if not already installed)
curl -L https://nixos.org/nix/install | sh
source ~/.nix-profile/etc/profile.d/nix.sh

## Enable flakes (add to ~/.config/nix/nix.conf)
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
```

#### 2. Build the Application
```bash
## Clone repository
git clone <repository-url> ~/s/site
cd ~/s/site

## Build with Nix
nix build

## The executable will be at: ./result/bin/site
```

#### 3. Create Systemd Service
```bash
## Create service file
sudo tee /etc/systemd/system/valeratrades.service > /dev/null <<EOF
[Unit]
Description=ValeRatrades Website
After=network.target

[Service]
Type=simple
User=$(whoami)
WorkingDirectory=$HOME/s/site
ExecStart=$HOME/s/site/result/bin/site
Restart=on-failure
RestartSec=5s
Environment="LEPTOS_SITE_ROOT=$HOME/s/site/target/site"

[Install]
WantedBy=multi-user.target
EOF

## Reload systemd and enable service
sudo systemctl daemon-reload
sudo systemctl enable valeratrades
sudo systemctl start valeratrades

## Check status
sudo systemctl status valeratrades
```

#### 4. Verify the Service
```bash
## Check if the site is running
curl http://127.0.0.1:61156

## View logs
sudo journalctl -u valeratrades -f
```

### Development Tips

#### Running Tests
```bash
## Run end-to-end tests
cd end2end
npx playwright test
```

#### Code Formatting
```bash
## Format code (runs automatically on git commit)
cargo fmt

## Check formatting
cargo fmt --check
```

#### Updating Dependencies
```bash
## Update Cargo dependencies
cargo update

## Update Nix flake inputs
nix flake update
```

### Troubleshooting

#### cargo-leptos Not Found
```bash
## Install manually
cargo install cargo-leptos
```

#### WASM Build Issues
```bash
## Ensure wasm32 target is installed
rustup target add wasm32-unknown-unknown
```

#### Port Already in Use
```bash
## Check what's using port 61156
sudo lsof -i :61156

## Kill the process if needed
kill -9 <PID>
```

#### Nix Build Fails
```bash
## Clear Nix cache and rebuild
nix-collect-garbage
nix build --rebuild
```

</details>
<!-- markdownlint-restore -->

## Usage
Instructions for deploying the site with HTTPS using nginx or Caddy, and configuring DNS.

### Table of Contents
- [Option 1: Nginx with Let's Encrypt (Recommended for existing nginx setups)](#option-1-nginx-with-lets-encrypt)
- [Option 2: Caddy (Easiest - automatic HTTPS)](#option-2-caddy)
- [DNS Configuration (Squarespace/Any Provider)](#dns-configuration)
- [Verification & Troubleshooting](#verification--troubleshooting)

---

### Option 1: Nginx with Let's Encrypt

Use this if nginx is already installed or you prefer more granular control.

#### Step 1: Install Nginx and Certbot
```sh
sudo apt update
sudo apt install -y nginx certbot python3-certbot-nginx
```

#### Step 2: Create Nginx Configuration
```sh
## Create site configuration
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

## Enable the site
sudo ln -sf /etc/nginx/sites-available/valeratrades.com /etc/nginx/sites-enabled/valeratrades.com

## Test configuration
sudo nginx -t

## Reload nginx
sudo systemctl reload nginx
```

#### Step 3: Obtain SSL Certificate
```sh
## Get certificate and auto-configure HTTPS
sudo certbot --nginx -d valeratrades.com -d www.valeratrades.com --non-interactive --agree-tos --register-unsafely-without-email --redirect

## Or with email for renewal notifications:
sudo certbot --nginx -d valeratrades.com -d www.valeratrades.com --non-interactive --agree-tos --email your-email@example.com --redirect
```

#### Step 4: Verify Auto-Renewal
```sh
## Test renewal process (dry run)
sudo certbot renew --dry-run

## Check certbot timer is enabled
sudo systemctl status certbot.timer
```

#### Managing Nginx

```sh
## Restart nginx
sudo systemctl restart nginx

## Reload nginx (no downtime)
sudo systemctl reload nginx

## View nginx logs
sudo tail -f /var/log/nginx/access.log
sudo tail -f /var/log/nginx/error.log

## Test configuration
sudo nginx -t

## View site configuration
cat /etc/nginx/sites-enabled/valeratrades.com
```

---

### Option 2: Caddy

Caddy automatically obtains and renews SSL certificates with zero configuration.

#### Step 1: Install Caddy
```sh
## Add Caddy repository
sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https curl
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list

## Install Caddy
sudo apt update
sudo apt install caddy
```

#### Step 2: Configure Caddy
```sh
## Create Caddyfile
sudo tee /etc/caddy/Caddyfile > /dev/null <<'EOF'
valeratrades.com {
    reverse_proxy localhost:61156
}

www.valeratrades.com {
    redir https://valeratrades.com{uri} permanent
}
EOF

## Reload Caddy
sudo systemctl reload caddy
```

That's it! Caddy automatically:
- Obtains SSL certificates from Let's Encrypt
- Renews certificates before expiration
- Redirects HTTP to HTTPS

#### Managing Caddy

```sh
## Restart Caddy
sudo systemctl restart caddy

## Reload configuration (no downtime)
sudo systemctl reload caddy

## View Caddy logs
sudo journalctl -u caddy -f

## Check Caddy status
sudo systemctl status caddy

## Validate Caddyfile syntax
caddy validate --config /etc/caddy/Caddyfile
```

---

### DNS Configuration

Configure DNS to point your domain to your server. This works for any DNS provider (Squarespace, Cloudflare, Route53, etc.).

#### For Squarespace DNS

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

#### For Other DNS Providers

**Cloudflare:**
1. Dashboard → Domain → DNS
2. Add A records for `@` and `www` pointing to your server IP
3. Set proxy status to "DNS only" (gray cloud) initially for testing
4. After verification, enable proxy (orange cloud) for DDoS protection and CDN

**Route53 (AWS):**
```sh
## Get your hosted zone ID
aws route53 list-hosted-zones

## Create A records
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

#### Finding Your Server IP

```sh
## On your server, run:
curl ifconfig.me

## Or:
ip addr show | grep 'inet ' | grep -v 127.0.0.1
```

#### Verify DNS Propagation

```sh
## Check if DNS is resolving
dig valeratrades.com
dig www.valeratrades.com

## Or use online tools:
## - https://dnschecker.org
## - https://www.whatsmydns.net
```

---

### Firewall Configuration

Ensure your firewall allows HTTP and HTTPS traffic:

```sh
## Using ufw (Ubuntu/Debian)
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw enable
sudo ufw status

## Using firewalld (RHEL/CentOS)
sudo firewall-cmd --permanent --add-service=http
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --reload

## Using iptables (manual)
sudo iptables -A INPUT -p tcp --dport 80 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 443 -j ACCEPT
sudo iptables-save
```

---

### Verification & Troubleshooting

#### Test HTTPS Connection
```sh
## Test from command line
curl -I https://valeratrades.com

## Expected output should include:
## HTTP/2 200 (or 301/302 for redirects)
## Connection should NOT show certificate errors
```

#### Test HTTP to HTTPS Redirect
```sh
curl -I http://valeratrades.com

## Expected: HTTP/1.1 301 Moved Permanently
## Location: https://valeratrades.com/
```

#### Check Certificate Details
```sh
## View certificate info
echo | openssl s_client -connect valeratrades.com:443 -servername valeratrades.com 2>/dev/null | openssl x509 -noout -dates -subject

## Or use online tools:
## https://www.ssllabs.com/ssltest/
```

#### Common Issues

##### 1. Certificate Not Found / Invalid
```sh
## For nginx:
sudo certbot certificates
sudo certbot renew --force-renewal

## For Caddy:
## Caddy manages this automatically, but check logs:
sudo journalctl -u caddy -f
```

##### 2. Site Not Accessible
```sh
## Check if your app is running
curl http://127.0.0.1:61156

## Check reverse proxy logs
## For nginx:
sudo tail -f /var/log/nginx/error.log

## For Caddy:
sudo journalctl -u caddy -f

## Check app logs
sudo journalctl -u valeratrades -f
```

##### 3. DNS Not Resolving
```sh
## Check DNS records
dig valeratrades.com
nslookup valeratrades.com

## Try different DNS servers
dig @8.8.8.8 valeratrades.com  # Google DNS
dig @1.1.1.1 valeratrades.com  # Cloudflare DNS

## Wait for propagation (can take up to 48 hours)
```

##### 4. 502 Bad Gateway
```sh
## Your reverse proxy can't reach the app
## Verify app is running:
sudo systemctl status valeratrades

## Restart app:
sudo systemctl restart valeratrades

## Check app logs:
sudo journalctl -u valeratrades -n 50
```

##### 5. Renewal Issues
```sh
## For nginx + certbot:
## Check renewal timer
sudo systemctl status certbot.timer

## Manually renew
sudo certbot renew

## For Caddy:
## Caddy handles renewal automatically
## Check logs if issues occur:
sudo journalctl -u caddy | grep -i renew
```

---

### Updating the Site

#### Standard Update Process
```sh
## 1. SSH into server
ssh cloudzy_ubuntu  # or your server alias

## 2. Navigate to site directory
cd ~/s/site

## 3. Pull latest changes
git pull

## 4. Rebuild
nix build --rebuild

## 5. Restart the service
sudo systemctl restart valeratrades

## 6. Verify it's working
curl -I https://valeratrades.com
```

#### Rolling Back
```sh
## If something breaks, revert to previous commit
git log --oneline  # Find the commit hash
git checkout <previous-commit-hash>
nix build --rebuild
sudo systemctl restart valeratrades
```

---

### Monitoring

#### View Application Logs
```sh
## Real-time logs
sudo journalctl -u valeratrades -f

## Last 100 lines
sudo journalctl -u valeratrades -n 100

## Logs from today
sudo journalctl -u valeratrades --since today

## Logs with errors only
sudo journalctl -u valeratrades -p err
```

#### Check Service Status
```sh
## Application status
sudo systemctl status valeratrades

## Nginx/Caddy status
sudo systemctl status nginx
## OR
sudo systemctl status caddy

## Check if ports are listening
sudo ss -tlnp | grep -E '(80|443|61156)'
```

#### Resource Usage
```sh
## Check CPU and memory
top
## Or:
htop

## Specific to your app
ps aux | grep site
```



<br>

<sup>
	This repository follows <a href="https://github.com/valeratrades/.github/tree/master/best_practices">my best practices</a> and <a href="https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/TIGER_STYLE.md">Tiger Style</a> (except "proper capitalization for acronyms": (VsrState, not VSRState) and formatting).
</sup>

#### License

<sup>
	Licensed under <a href="LICENSE">Blue Oak 1.0.0</a>
</sup>

<br>

<sub>
	Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be licensed as above, without any additional terms or conditions.
</sub>

