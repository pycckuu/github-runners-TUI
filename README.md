# GitHub Self-Hosted Runners for Igra Labs

Reduce GitHub Actions costs by **60-80%** with automated self-hosted runner setup and management.

## 🚀 Quick Start

```bash
# Clone and setup
git clone <this-repo-url>
cd gh-runners
chmod +x *.sh

# Setup build environment and Rust (recommended)
./setup_build_environment.sh

# Setup 4 runners for your repository
./setup_runners.sh myorg/myrepo 4
```

## ✨ What You Get

- **💰 Cost Savings**: No more per-minute billing for GitHub Actions
- **🏗️ Build Environment**: Complete Rust, C/C++, and system tools setup
- **📦 Multiple Repositories**: Manage runners across different repos
- **🔧 Easy Management**: Simple commands for start/stop/restart/logs
- **⚙️ Auto-Service Installation**: Runs as systemd services with health monitoring
- **🚀 CI Stability**: Prevents common CI issues and environment mismatches

## 📋 Requirements

- Linux machine with 4+ GB RAM, 50+ GB storage
- `sudo` access and internet connection
- GitHub repository admin access
- Standard Linux utilities (`curl`, `tar`, `systemctl`, etc.)

## 🎯 Basic Usage

### Initial Setup
```bash
# Get GitHub token: Repo Settings → Actions → Runners → "New self-hosted runner"
./setup_runners.sh myorg/myrepo 4

# Optional: Setup complete build environment
./setup_build_environment.sh
```

### Daily Management
```bash
# Manage specific repository
./manage_repo.sh myrepo status
./manage_repo.sh myrepo restart
./manage_repo.sh myrepo logs

# Manage all runners
./manage_all.sh status
./list_repos.sh

# Health check
./health_check.sh
```

## 🔧 Build Environment Features

The included `setup_build_environment.sh` provides:

- **System Build Tools**: `build-essential`, `pkg-config`, `libssl-dev`, LLVM/Clang
- **Rust Environment**: Stable + nightly toolchains, rustfmt, clippy, miri
- **CI Tools**: `cargo-audit`, `typos-cli`, proper environment configuration
- **Stability Fixes**: Prevents version mismatches, concurrent conflicts, state pollution

⚠️ **Recommended**: Run the build environment setup for reliable CI/CD.

## 🔗 Use in Workflows

### Basic Usage
```yaml
jobs:
  build:
    runs-on: [self-hosted, myrepo]  # Use your repo name as label
    steps:
      - uses: actions/checkout@v4
      - run: cargo build --release
```

### With Rust Environment
```yaml
jobs:
  test:
    runs-on: [self-hosted, myrepo]
    steps:
      - name: Setup Rust environment
        run: |
          source ~/.cargo/env
          echo "$HOME/.cargo/bin" >> $GITHUB_PATH

      - uses: actions/checkout@v4
      - run: cargo test --release
```


## 🛠️ Available Scripts

| Script | Purpose |
|:-------|:--------|
| `setup_runners.sh` | Create and configure runners for a repository |
| `setup_build_environment.sh` | Install comprehensive build tools and Rust |
| `manage_repo.sh` | Manage runners for specific repository |
| `manage_all.sh` | Manage all runners across repositories |
| `list_repos.sh` | Show repository and runner status |
| `health_check.sh` | Check runner health and system resources |
| `update_all.sh` | Update all runners to latest version |

## 🔍 Troubleshooting

### Common Issues
- **"linker 'cc' not found"**: Run `./setup_build_environment.sh`
- **"cargo-fmt not found"**: Rust installation issue, see setup guide
- **Runners not starting**: Check with `./manage_repo.sh myrepo debug`

### Get Help
```bash
# View logs
./manage_repo.sh myrepo logs

# Check system resources
./health_check.sh

# Debug services
./manage_repo.sh myrepo debug
```

## 📚 Documentation

**→ [Complete Setup Guide](doc/setup-guide.md)** - Detailed installation, configuration, troubleshooting, workflow examples, and optimization guide.

