# Installation

Full setup instructions to get the site running locally or on a server.

## Prerequisites

### System Requirements
- NixOS or a system with Nix package manager
- Git

## Local Development Setup

### 1. Clone the Repository
```bash
git clone <repository-url>
cd site
```

### 2. Enter Nix Development Shell
```bash
nix develop
```

This will automatically:
- Install Rust nightly toolchain with wasm32-unknown-unknown target
- Install cargo-leptos (or update to latest version)
- Install frontend build tools (sassc, binaryen, tailwindcss)
- Build Tailwind CSS
- Set up git hooks and pre-commit checks

### 3. Build and Run Locally
```bash
# Watch mode with hot reload (recommended for development)
lw  # alias for: cargo leptos watch --hot-reload

# Or manually:
cargo leptos watch --hot-reload
```

The site will be available at `http://127.0.0.1:61156`

### 4. Production Build
```bash
# Build Tailwind CSS first
tailwindcss -i ./style/tailwind_in.css -o ./style/tailwind_out.css

# Build with Nix (recommended)
nix build

# The binary will be in: ./result/bin/site
./result/bin/site

# Or build with cargo-leptos
cargo leptos build --release
```

## Server Deployment Setup

### 1. Install Dependencies on Server
```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install required packages
sudo apt install -y build-essential curl git

# Install Nix (if not already installed)
curl -L https://nixos.org/nix/install | sh
source ~/.nix-profile/etc/profile.d/nix.sh

# Enable flakes (add to ~/.config/nix/nix.conf)
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
```

### 2. Build the Application
```bash
# Clone repository
git clone <repository-url> ~/s/site
cd ~/s/site

# Build with Nix
nix build

# The executable will be at: ./result/bin/site
```

### 3. Create Systemd Service
```bash
# Create service file
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

# Reload systemd and enable service
sudo systemctl daemon-reload
sudo systemctl enable valeratrades
sudo systemctl start valeratrades

# Check status
sudo systemctl status valeratrades
```

### 4. Verify the Service
```bash
# Check if the site is running
curl http://127.0.0.1:61156

# View logs
sudo journalctl -u valeratrades -f
```

## Development Tips

### Running Tests
```bash
# Run end-to-end tests
cd end2end
npx playwright test
```

### Code Formatting
```bash
# Format code (runs automatically on git commit)
cargo fmt

# Check formatting
cargo fmt --check
```

### Updating Dependencies
```bash
# Update Cargo dependencies
cargo update

# Update Nix flake inputs
nix flake update
```

## Troubleshooting

### cargo-leptos Not Found
```bash
# Install manually
cargo install cargo-leptos
```

### WASM Build Issues
```bash
# Ensure wasm32 target is installed
rustup target add wasm32-unknown-unknown
```

### Port Already in Use
```bash
# Check what's using port 61156
sudo lsof -i :61156

# Kill the process if needed
kill -9 <PID>
```

### Nix Build Fails
```bash
# Clear Nix cache and rebuild
nix-collect-garbage
nix build --rebuild
```
