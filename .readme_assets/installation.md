## Prerequisites
- Nix package manager with flakes enabled
- Git

## Local Development
```bash
git clone <repository-url> && cd site
nix develop
lw  # alias for: cargo leptos watch --hot-reload
```
Site available at `http://127.0.0.1:61156`

## Production Build
```bash
nix build
./result/bin/site
```

## Server Deployment
```bash
# Install Nix, enable flakes, then:
git clone <repository-url> ~/s/site && cd ~/s/site
nix build

# Create systemd service at /etc/systemd/system/valeratrades.service:
# [Unit]
# Description=ValeRatrades Website
# After=network.target
# [Service]
# Type=simple
# User=<user>
# WorkingDirectory=/home/<user>/s/site
# ExecStart=/home/<user>/s/site/result/bin/site
# Restart=on-failure
# Environment="LEPTOS_SITE_ROOT=/home/<user>/s/site/target/site"
# [Install]
# WantedBy=multi-user.target

sudo systemctl daemon-reload && sudo systemctl enable --now valeratrades
```

## Troubleshooting
- **cargo-leptos not found**: `cargo install cargo-leptos`
- **WASM issues**: `rustup target add wasm32-unknown-unknown`
- **Port in use**: `sudo lsof -i :61156`
- **Nix build fails**: `nix-collect-garbage && nix build --rebuild`
