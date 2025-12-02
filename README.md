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

### Prerequisites
- Nix package manager with flakes enabled
- Git

### Local Development
```sh
git clone <repository-url> && cd site
nix develop
lw  # alias for: cargo leptos watch --hot-reload
```
Site available at `http://127.0.0.1:61156`

### Production Build
```sh
nix build
./result/bin/site
```

### Server Deployment
```sh
git clone <repository-url> ~/s/site && cd ~/s/site
nix build

sudo tee /etc/systemd/system/valeratrades.service > /dev/null <<EOF
[Unit]
Description=ValeRatrades Website
After=network.target
[Service]
Type=simple
User=$USER
WorkingDirectory=$HOME/s/site
ExecStart=$HOME/s/site/result/bin/site
Restart=on-failure
Environment="LEPTOS_SITE_ROOT=$HOME/s/site/target/site"
[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload && sudo systemctl enable --now valeratrades
```

### Troubleshooting
- **cargo-leptos not found**: `cargo install cargo-leptos`
- **WASM issues**: `rustup target add wasm32-unknown-unknown`
- **Port in use**: `sudo lsof -i :61156`
- **Nix build fails**: `nix-collect-garbage && nix build --rebuild`

</details>
<!-- markdownlint-restore -->

## Usage
### HTTPS Setup

**Caddy** (recommended): Auto-manages SSL certificates. Install, add `valeratrades.com { reverse_proxy localhost:61156 }` to `/etc/caddy/Caddyfile`, reload.

**Nginx + Certbot**: Install nginx and certbot, configure reverse proxy to `127.0.0.1:61156`, run `sudo certbot --nginx -d valeratrades.com`.

### DNS
Add A records for `@` and `www` pointing to your server IP.

### Updating
```bash
cd ~/s/site && git pull && nix build --rebuild && sudo systemctl restart valeratrades
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

