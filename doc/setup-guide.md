# GitHub Self-Hosted Runners Setup Script Documentation

This documentation explains how to use the GitHub runner setup script to create multiple self-hosted runners for your repository, helping reduce CI/CD costs and improve build performance.

## Prerequisites

### System Requirements
- Linux machine with at least 4 GB RAM and 50 GB free disk space
- Internet connection for downloading runner packages
- `sudo` access for installing services
- `curl` and `tar` utilities installed
- `awk` utility installed
- `journalctl` utility installed
- `htop` utility installed
- `df` utility installed
- `ps` utility installed
- `grep` utility installed
- `systemctl` utility installed
- `journalctl` utility installed

### GitHub Repository Access
- Repository admin access or organization permissions to add self-hosted runners
- Access to generate runner registration tokens

## Installation and Setup

### 1. Create the Base Directory
```bash
mkdir -p ~/action-runners
cd ~/action-runners
```

### 2. Create the Setup Script

Use the existing setup script in the repository:

```bash
chmod +x setup_runners.sh
./setup_runners.sh myorg/myrepo 4
```

This script handles the complete setup of multiple GitHub self-hosted runners, including downloading, configuring, and optionally installing them as system services.

### 3. Set Up Build Environment

Run the build environment setup script:

```bash
chmod +x setup_build_environment.sh
./setup_build_environment.sh
```

This script installs essential build tools including `build-essential`, `pkg-config`, `libssl-dev`, and other dependencies required for compiling Rust projects.

### 4. Set Up Rust Environment

Run the comprehensive Rust setup script:

```bash
chmod +x setup_rust_environment.sh
./setup_rust_environment.sh
```

This script:
- Installs Rust via rustup with the stable toolchain
- Adds rustfmt and clippy components
- Installs nightly toolchain with miri for undefined behavior detection
- Installs useful cargo tools like `cargo-audit` and `typos-cli`
- Configures the environment in your shell profile

### 5. Create Management Scripts

The repository includes several management scripts for operating your runners:

#### Repository-Specific Management Script

```bash
chmod +x manage_repo.sh
```

This script allows you to manage all runners for a specific repository with commands like `start`, `stop`, `restart`, `status`, `logs`, and `debug`.

#### Global Management Script

```bash
chmod +x manage_all.sh
```

This script manages ALL GitHub runners on the system across all repositories.

#### Repository Listing Script

```bash
chmod +x list_repos.sh
```

This script shows all repositories and their runner status.

#### Health Check Script

```bash
chmod +x health_check.sh
```

This script performs health checks on all runners and reports their status.

## Usage Guide

### Setting Up Runners for a Repository

#### 1. Get Your GitHub Registration Token

1. **Navigate to your repository on GitHub**
2. **Go to Settings → Actions → Runners**
3. **Click "New self-hosted runner"**
4. **Copy the token** from the configuration command shown

⚠️ **Important:** Tokens expire after 1 hour. Generate a new token if setup takes longer.

#### 2. Run the Setup Script

```bash
cd ~/action-runners

# Basic usage (sets up 4 runners by default)
./setup_runners.sh myorg/viaduct

# With custom number of runners (e.g., 6 runners)
./setup_runners.sh myorg/api-service 6
```

#### 3. Directory Structure After Setup

```
~/action-runners/
├── setup_runners.sh
├── setup_build_environment.sh
├── setup_rust_environment.sh
├── manage_repo.sh
├── manage_all.sh
├── list_repos.sh
├── health_check.sh
├── viaduct/
│   ├── 1/
│   ├── 2/
│   ├── 3/
│   └── 4/
└── api-service/
    ├── 1/
    ├── 2/
    ├── 3/
    └── 4/
```

### Managing Your Runners

#### List All Repositories and Runner Status

```bash
./list_repos.sh
```

#### Repository-Specific Management

```bash
# Check status of all runners for 'viaduct' repository
./manage_repo.sh viaduct status

# Stop all runners for 'viaduct' repository
./manage_repo.sh viaduct stop

# Start all runners for 'viaduct' repository
./manage_repo.sh viaduct start

# View live logs for 'viaduct' runners
./manage_repo.sh viaduct logs

# Debug service discovery
./manage_repo.sh viaduct debug
```

#### Global Management

```bash
# Check status of ALL runners
./manage_all.sh status

# Stop ALL runners
./manage_all.sh stop

# Start ALL runners
./manage_all.sh start

# View live logs from ALL runners
./manage_all.sh logs
```

## Workflow Configuration

### Enhanced Rust Workflow Template

```yaml
name: Format, Lint & Typos

on:
  push:
    branches: [main]
  pull_request:

jobs:
  format-lint-typos:
    runs-on: [self-hosted, viaduct]  # Use your repository label
    steps:
      - name: Setup Rust environment
        run: |
          # Source Rust environment
          source ~/.cargo/env

          # Add to GitHub Actions PATH for all subsequent steps
          echo "$HOME/.cargo/bin" >> $GITHUB_PATH

          # Set environment variables
          echo "CARGO_HOME=$HOME/.cargo" >> $GITHUB_ENV
          echo "RUSTUP_HOME=$HOME/.rustup" >> $GITHUB_ENV

          # Verify Rust is working
          echo "=== Rust Environment ==="
          echo "rustup: $(rustup --version)"
          echo "rustc: $(rustc --version)"
          echo "cargo: $(cargo --version)"
          echo "========================"

      - uses: actions/checkout@v4

      - name: Cache cargo dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-

      - name: Check formatting
        run: cargo fmt -- --check

      - name: Run clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Install and run typos
        run: |
          if ! command -v typos &> /dev/null; then
            cargo install typos-cli
          fi
          typos
```

### Build and Test Workflow

```yaml
name: Build and Test

on:
  push:
    branches: [main]
  pull_request:

jobs:
  build-and-test:
    runs-on: [self-hosted, viaduct]
    steps:
      - name: Setup Rust environment
        run: |
          source ~/.cargo/env
          echo "$HOME/.cargo/bin" >> $GITHUB_PATH
          echo "CARGO_HOME=$HOME/.cargo" >> $GITHUB_ENV
          echo "RUSTUP_HOME=$HOME/.rustup" >> $GITHUB_ENV

      - uses: actions/checkout@v4

      - name: Cache cargo dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Build
        run: |
          # Use all available cores for faster builds
          export CARGO_BUILD_JOBS=$(nproc)
          cargo build --release --verbose

      - name: Run tests
        run: |
          # Run tests with all available cores
          export CARGO_TEST_THREADS=$(nproc)
          cargo test --release --verbose
```

### Dependency Audit Workflow

```yaml
name: Dependency security audit

on:
  push:
    paths:
      - "**/Cargo.toml"
      - "**/Cargo.lock"
  schedule:
    - cron: '0 2 * * 1'  # Weekly on Monday at 2 AM (new vulnerabilities)

env:
  CARGO_TERM_COLOR: always

jobs:
  security_audit:
    timeout-minutes: 10
    runs-on: [self-hosted, viaduct]  # Add specific labels
    permissions:
      contents: read
      checks: write
    steps:
      - name: Setup Rust environment  # Add this step
        run: |
          source ~/.cargo/env
          echo "$HOME/.cargo/bin" >> $GITHUB_PATH

      - name: Check out
        uses: actions/checkout@v4

      - name: Cache audit-check build
        id: cache-audit-check
        uses: actions/cache@v4
        continue-on-error: false
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: ${{ runner.os }}-cargo-

      - name: Run audit-check action
        run: |
          which cargo-deny || cargo install cargo-deny
          cargo deny check
```

### UB Detection Workflow

```yaml
name: Undefined Behavior Detection

on:
  push:
    branches: [main]
  pull_request:

jobs:
  miri:
    runs-on: [self-hosted, viaduct]
    steps:
      - name: Setup Rust nightly environment
        run: |
          source ~/.cargo/env
          echo "$HOME/.cargo/bin" >> $GITHUB_PATH

          # Ensure nightly and miri are available
          rustup toolchain install nightly
          rustup component add miri --toolchain nightly

      - uses: actions/checkout@v4

      - name: Run Miri
        run: cargo +nightly miri test
```

## Troubleshooting

### Common Issues and Solutions

#### 1. "linker 'cc' not found" Error

**Problem:** Missing C compiler and build tools.

**Solution:**
```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev
cd ~/action-runners
./manage_all.sh restart
```

#### 2. "cargo-fmt is not installed" Error

**Problem:** Rust components missing or broken installation.

**Solution:**
```bash
# Check Rust installation
ls -la ~/.cargo/bin/

# If symlinks are broken (pointing to missing rustup):
rm -rf ~/.cargo ~/.rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env
rustup component add rustfmt clippy
```

#### 3. Broken Rust Symlinks

**Problem:** All tools in `~/.cargo/bin/` are symlinks pointing to missing `rustup`.

**Diagnosis:**
```bash
ls -la ~/.cargo/bin/
# Shows: cargo -> rustup, rustc -> rustup, etc.
which rustup
# Shows: rustup not found
```

**Solution:**
```bash
# Complete reinstallation
rm -rf ~/.cargo ~/.rustup
./setup_rust_environment.sh
```

#### 4. Runners Not Found by Management Scripts

**Problem:** Service names don't match expected patterns.

**Solution:**
```bash
# Debug service names
sudo systemctl list-units --type=service | grep actions.runner

# Use the debug function
./manage_repo.sh viaduct debug
```

#### 5. Environment Not Persisting in Workflows

**Problem:** Rust commands work in shell but not in GitHub Actions.

**Solution:** Always add this to your workflow steps:
```yaml
- name: Setup Rust environment
  run: |
    source ~/.cargo/env
    echo "$HOME/.cargo/bin" >> $GITHUB_PATH
```

### Log Analysis

#### View Detailed Logs
```bash
# Repository-specific logs
./manage_repo.sh viaduct logs

# All runners logs
./manage_all.sh logs

# Historical logs
sudo journalctl -u actions.runner.* --since "1 hour ago"
```

#### Log Cleanup
```bash
# Clean old logs
sudo journalctl --vacuum-time=2weeks
sudo journalctl --vacuum-size=1G
```

### Health Monitoring

#### Manual Health Check
```bash
./health_check.sh
```

#### Automated Health Monitoring
```bash
# Set up cron job for health checks
crontab -e
# Add: */5 * * * * /home/$(whoami)/action-runners/health_check.sh >> /var/log/runner-health.log 2>&1
```

### Update All Runners
```bash
cat > ~/action-runners/update_all.sh << 'EOF'
#!/bin/bash
cd ~/action-runners
for repo_dir in */; do
    if [ -d "$repo_dir" ] && [ "$repo_dir" != "*/" ]; then
        repo_name=${repo_dir%/}
        echo "Updating runners for $repo_name..."

        for runner_dir in "$repo_dir"*/; do
            if [ -d "$runner_dir" ] && [ -f "$runner_dir/config.sh" ]; then
                cd "$runner_dir"
                sudo ./svc.sh stop
                ./bin/Runner.Listener update
                sudo ./svc.sh start
                cd ~/action-runners
            fi
        done
    fi
done
EOF

chmod +x ~/action-runners/update_all.sh
```

## Performance Optimization

### Hardware Recommendations

| Repository Type | Runners | CPU per Runner | RAM per Runner | Storage |
|:----------------|:--------|:---------------|:---------------|:--------|
| Small projects | 2-4 | 2 cores | 4 GB | 20 GB |
| Medium projects | 4-6 | 4 cores | 8 GB | 50 GB |
| Large projects | 6-8 | 8 cores | 16 GB | 100 GB |
| Enterprise | 8+ | 8+ cores | 16+ GB | 200+ GB |

### Cost Savings

With self-hosted runners, you'll save on:
- ✅ GitHub Actions minutes (no per-minute billing)
- ✅ Build queue times (dedicated resources)
- ✅ Data transfer costs (local artifact storage)
- ✅ Custom hardware optimization

Expected savings: **60-80%** reduction in CI/CD costs for most projects.

### Resource Optimization

```bash
# Monitor system resources
htop
df -h ~/action-runners/

# Check runner-specific resource usage
ps aux | grep Runner.Listener

# Optimize concurrent job processing
# Edit .runner.jitconfig in each runner directory:
echo '{"workJobConcurrency":"4"}' > ~/.../runner/.runner.jitconfig
```

This comprehensive setup gives you full control over your GitHub Actions infrastructure while significantly reducing costs and improving performance.

