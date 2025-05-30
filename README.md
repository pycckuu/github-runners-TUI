# GitHub Self-Hosted Runners for Igra Labs

Reduce GitHub Actions costs by **60-80%** with automated self-hosted runner setup and management.

## 🚀 Quick Start

```bash
# Clone and setup
git clone <this-repo-url>
cd gh-runners
chmod +x *.sh

# Setup 4 runners for your repository
./setup_runners.sh myorg/myrepo 4
```

## ✨ What You Get

- **Cost Savings**: No more per-minute billing for GitHub Actions
- **Multiple Repositories**: Manage runners across different repos
- **Easy Management**: Simple commands for start/stop/restart
- **Auto-Service Installation**: Runs as systemd services
- **Health Monitoring**: Built-in status checks

## 📋 Requirements

- Linux machine with 4+ GB RAM, 50+ GB storage
- `sudo` access and internet connection
- GitHub repository admin access

## 🎯 Basic Usage

```bash
# Manage specific repository
./manage_repo.sh myrepo status
./manage_repo.sh myrepo restart

# Manage all runners
./manage_all.sh status
./list_repos.sh

# Health check
./health_check.sh
```

## 🔗 Use in Workflows

```yaml
jobs:
  build:
    runs-on: self-hosted
    steps:
      - uses: actions/checkout@v4
      - run: make build
```

## 📚 Documentation

**→ [Complete Setup Guide](doc/setup-guide.md)** - Detailed installation, configuration, troubleshooting, and optimization guide.

## 🤝 Support

⭐ Star this repo if it saves you money!
🐛 Report issues in GitHub Issues
📖 Full documentation in [doc/setup-guide.md](doc/setup-guide.md)
