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

Save this script as `setup_runners.sh`:

```bash
cat > ~/action-runners/setup_runners.sh << 'EOF'
#!/bin/bash
set -e

# Check if required arguments are provided
if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <github-repo> [num-runners]"
    echo "Example: $0 myorg/myrepo 4"
    echo "Note: If number of runners is not provided, 4 will be used as default."
    exit 1
fi

# Configuration variables
GITHUB_REPO=$1
NUM_RUNNERS=${2:-4}  # Use the second argument or default to 4 runners
REPO_NAME=$(echo "$GITHUB_REPO" | cut -d'/' -f2)  # Extract repo name from org/repo
REPO_URL="https://github.com/${GITHUB_REPO}"
BASE_DIR="$HOME/action-runners/${REPO_NAME}"
RUNNER_VERSION="2.314.1"  # Update this to the latest version if needed
ARCHITECTURE="x64"  # Change to arm64 if needed
OS="linux"          # Change to darwin for macOS or win for Windows

echo "Setting up ${NUM_RUNNERS} runners for repository: ${GITHUB_REPO}"
echo "Using base directory: ${BASE_DIR}"

# Function to set up a runner
setup_runner() {
    local runner_number=$1
    local runner_dir="${BASE_DIR}/${runner_number}"
    local runner_name="${REPO_NAME}-runner-${runner_number}"

    echo "Setting up runner ${runner_number} in ${runner_dir}..."

    # Create directory if it doesn't exist
    mkdir -p "${runner_dir}"

    # Enter the runner directory
    cd "${runner_dir}"

    # Download the runner if not already present
    if [ ! -f "actions-runner-${OS}-${ARCHITECTURE}.tar.gz" ]; then
        echo "Downloading runner package..."
        curl -o "actions-runner-${OS}-${ARCHITECTURE}.tar.gz" -L "https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-${OS}-${ARCHITECTURE}-${RUNNER_VERSION}.tar.gz"
    fi

    # Extract the runner if not already extracted
    if [ ! -f "./config.sh" ]; then
        echo "Extracting runner package..."
        tar xzf "./actions-runner-${OS}-${ARCHITECTURE}.tar.gz"
    fi

    # Check for existing runner configuration
    if [ -f "./.runner" ]; then
        echo "Runner ${runner_number} is already configured. Skipping configuration."
        return 0
    fi

    # Prompt for token if not already provided
    if [ -z "$TOKEN" ]; then
        echo "Please enter your GitHub runner registration token:"
        read -r TOKEN
    fi

    # Configure the runner
    echo "Configuring runner ${runner_number}..."
    ./config.sh --url "$REPO_URL" --token "$TOKEN" --name "$runner_name" \
                --labels "self-hosted,linux,x64,${REPO_NAME}" --work "_work" --unattended

    # Set up concurrent job processing (optional)
    echo '{"workJobConcurrency":"2"}' > .runner.jitconfig

    # Install as a service if user wants to
    echo "Do you want to install runner ${runner_number} as a service? (y/n)"
    read -r install_service

    if [[ "$install_service" == "y" || "$install_service" == "Y" ]]; then
        sudo ./svc.sh install
        sudo ./svc.sh start
        echo "Runner ${runner_number} installed and started as a service."
    else
        echo "To start the runner manually, use: cd ${runner_dir} && ./run.sh"
    fi
}

# Main script
echo "Setting up ${NUM_RUNNERS} GitHub runners for repository..."

# Create directories if they don't exist
for i in $(seq 1 $NUM_RUNNERS); do
    mkdir -p "${BASE_DIR}/${i}"
done

# Prompt for GitHub token
echo "Please enter your GitHub runner registration token:"
read -r TOKEN

# Set up each runner
for i in $(seq 1 $NUM_RUNNERS); do
    setup_runner "$i"
done

echo "All ${NUM_RUNNERS} runners have been set up!"
echo "You can verify they are connected in your GitHub repository settings under Actions > Runners."
echo ""
echo "To manage your runners:"
echo "- View status: Visit your repository's Settings > Actions > Runners page"
echo "- Update runners: cd into each runner directory and run ./bin/Runner.Listener update"
echo "- Remove runners: cd into each runner directory and run ./config.sh remove --token YOUR_REMOVAL_TOKEN"

# Provide instructions for workflow usage
echo ""
echo "To use these runners in your workflows, add this to your .github/workflows/*.yml files:"
echo "jobs:"
echo "  your_job_name:"
echo "    runs-on: self-hosted"
echo "    # Or specifically target your runners with:"
echo "    # runs-on: [self-hosted, ${REPO_NAME}]"
EOF

chmod +x ~/action-runners/setup_runners.sh
```

### 3. Create Management Scripts

#### Repository-Specific Management Script (`manage_repo.sh`)

```bash
cat > ~/action-runners/manage_repo.sh << 'EOF'
#!/bin/bash

if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <repo-name> <action>"
    echo "Actions: start, stop, restart, status, logs, debug"
    echo "Example: $0 viaduct status"
    exit 1
fi

REPO_NAME=$1
ACTION=$2

# Function to find services for a repository
find_repo_services() {
    local repo=$1
    # Look for services that contain the repo name and end with -runner-[number]
    sudo systemctl list-units --all --type=service | grep "actions.runner.*${repo}-runner-" | awk '{print $1}'
}

# Function to debug service discovery
debug_services() {
    echo "Debug information for repository: $REPO_NAME"
    echo "Looking for services containing: ${REPO_NAME}-runner-"
    echo ""

    echo "All GitHub Actions runner services on this system:"
    sudo systemctl list-units --all --type=service | grep actions.runner || echo "No GitHub Actions runner services found"
    echo ""

    echo "Services for repository '$REPO_NAME':"
    local services=$(find_repo_services "$REPO_NAME")
    if [ -z "$services" ]; then
        echo "No services found for repository '$REPO_NAME'"
        echo ""
        echo "Possible issues:"
        echo "1. Runners were not installed as services (answered 'n' during setup)"
        echo "2. Service names are different than expected"
        echo "3. Repository name doesn't match the service naming pattern"
        echo ""
        echo "Runner directories found:"
        ls ~/action-runners/$REPO_NAME/*/run.sh 2>/dev/null || echo "No runner directories found"
    else
        echo "$services"
    fi
}

case $ACTION in
    start|stop|restart)
        services=$(find_repo_services "$REPO_NAME")
        if [ -z "$services" ]; then
            echo "No services found for repository: $REPO_NAME"
            echo "Run '$0 $REPO_NAME debug' for more information"
            exit 1
        fi

        echo "${ACTION^}ing runners for repository: $REPO_NAME"
        for service in $services; do
            echo "  $ACTION $service"
            sudo systemctl $ACTION "$service"
        done
        ;;
    status)
        services=$(find_repo_services "$REPO_NAME")
        if [ -z "$services" ]; then
            echo "No services found for repository: $REPO_NAME"
            echo "Run '$0 $REPO_NAME debug' for more information"
            exit 1
        fi

        echo "Status for repository: $REPO_NAME"
        for service in $services; do
            echo ""
            echo "Service: $service"
            sudo systemctl status "$service" --no-pager -l
        done
        ;;
    logs)
        services=$(find_repo_services "$REPO_NAME")
        if [ -z "$services" ]; then
            echo "No services found for repository: $REPO_NAME"
            echo "Run '$0 $REPO_NAME debug' for more information"
            exit 1
        fi

        echo "Following logs for repository: $REPO_NAME (Ctrl+C to exit)"
        service_args=""
        for service in $services; do
            service_args="$service_args -u $service"
        done
        sudo journalctl -f $service_args
        ;;
    debug)
        debug_services
        ;;
    *)
        echo "Invalid action: $ACTION"
        echo "Valid actions: start, stop, restart, status, logs, debug"
        exit 1
        ;;
esac
EOF

chmod +x ~/action-runners/manage_repo.sh
```

#### Global Management Script (`manage_all.sh`)

```bash
cat > ~/action-runners/manage_all.sh << 'EOF'
#!/bin/bash

if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <action>"
    echo "Actions: start, stop, restart, status, logs, list"
    echo "Example: $0 status"
    exit 1
fi

ACTION=$1

# Function to find all GitHub runner services
find_all_runner_services() {
    sudo systemctl list-units --all --type=service | grep "actions.runner" | awk '{print $1}'
}

case $ACTION in
    start|stop|restart)
        services=$(find_all_runner_services)
        if [ -z "$services" ]; then
            echo "No GitHub runner services found"
            exit 1
        fi

        echo "${ACTION^}ing ALL GitHub runners..."
        for service in $services; do
            echo "  $ACTION $service"
            sudo systemctl $ACTION "$service"
        done
        ;;
    status)
        services=$(find_all_runner_services)
        if [ -z "$services" ]; then
            echo "No GitHub runner services found"
            exit 1
        fi

        echo "Status of ALL GitHub runners:"
        for service in $services; do
            echo ""
            echo "Service: $service"
            sudo systemctl status "$service" --no-pager -l
        done
        ;;
    logs)
        services=$(find_all_runner_services)
        if [ -z "$services" ]; then
            echo "No GitHub runner services found"
            exit 1
        fi

        echo "Following logs for ALL GitHub runners (Ctrl+C to exit):"
        service_args=""
        for service in $services; do
            service_args="$service_args -u $service"
        done
        sudo journalctl -f $service_args
        ;;
    list)
        ./list_repos.sh
        ;;
    *)
        echo "Invalid action: $ACTION"
        echo "Valid actions: start, stop, restart, status, logs, list"
        exit 1
        ;;
esac
EOF

chmod +x ~/action-runners/manage_all.sh
```

#### Repository Listing Script (`list_repos.sh`)

```bash
cat > ~/action-runners/list_repos.sh << 'EOF'
#!/bin/bash
echo "GitHub Runner Repositories and Services:"
echo "========================================"

cd ~/action-runners
for repo_dir in */; do
    if [ -d "$repo_dir" ] && [ "$repo_dir" != "*/" ]; then
        repo_name=${repo_dir%/}
        echo ""
        echo "Repository: $repo_name"
        echo "Runners:"

        runner_count=0
        for runner_dir in "$repo_dir"*/; do
            if [ -d "$runner_dir" ]; then
                runner_num=${runner_dir%/}
                runner_num=${runner_num##*/}

                # Find the actual service name for this runner
                service_name=$(sudo systemctl list-units --all --type=service | grep "actions.runner.*${repo_name}-runner-${runner_num}" | awk '{print $1}')

                if [ -n "$service_name" ]; then
                    status=$(sudo systemctl is-active "$service_name" 2>/dev/null || echo "unknown")
                    echo "  - Runner $runner_num ($service_name): $status"
                else
                    echo "  - Runner $runner_num: not-a-service (manual start required)"
                fi
                ((runner_count++))
            fi
        done
        echo "  Total runners: $runner_count"
    fi
done
EOF

chmod +x ~/action-runners/list_repos.sh
```

#### Health Check Script (`health_check.sh`)

```bash
cat > ~/action-runners/health_check.sh << 'EOF'
#!/bin/bash

echo "GitHub Runners Health Check"
echo "=========================="
echo "Timestamp: $(date)"
echo ""

total_runners=0
active_runners=0
failed_runners=0

cd ~/action-runners
for repo_dir in */; do
    if [ -d "$repo_dir" ] && [ "$repo_dir" != "*/" ]; then
        repo_name=${repo_dir%/}

        for runner_dir in "$repo_dir"*/; do
            if [ -d "$runner_dir" ]; then
                runner_num=${runner_dir%/}
                runner_num=${runner_num##*/}

                # Find the actual service name for this runner
                service_name=$(sudo systemctl list-units --all --type=service | grep "actions.runner.*${repo_name}-runner-${runner_num}" | awk '{print $1}')

                if [ -n "$service_name" ]; then
                    ((total_runners++))

                    status=$(sudo systemctl is-active "$service_name" 2>/dev/null)
                    if [ "$status" = "active" ]; then
                        ((active_runners++))
                    else
                        ((failed_runners++))
                        echo "❌ FAILED: $service_name ($status)"
                    fi
                fi
            fi
        done
    fi
done

echo ""
echo "Summary:"
echo "  Total runners: $total_runners"
echo "  Active runners: $active_runners"
echo "  Failed runners: $failed_runners"

if [ $failed_runners -eq 0 ]; then
    echo "✅ All runners are healthy!"
    exit 0
else
    echo "⚠️  Some runners need attention!"
    exit 1
fi
EOF

chmod +x ~/action-runners/health_check.sh
```

#### Service Installation Helper (`install_services.sh`)

```bash
cat > ~/action-runners/install_services.sh << 'EOF'
#!/bin/bash

if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <repo-name>"
    echo "Example: $0 viaduct"
    exit 1
fi

REPO_NAME=$1

echo "Installing services for repository: $REPO_NAME"

cd ~/action-runners/$REPO_NAME

for runner_dir in */; do
    if [ -d "$runner_dir" ] && [ -f "$runner_dir/svc.sh" ]; then
        runner_num=${runner_dir%/}
        echo "Installing service for runner $runner_num..."

        cd "$runner_dir"
        sudo ./svc.sh install
        sudo ./svc.sh start
        cd ..

        echo "✅ Runner $runner_num installed and started as service"
    fi
done

echo "All services installed for repository: $REPO_NAME"
EOF

chmod +x ~/action-runners/install_services.sh
```

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

# Set up 2 runners for a different repo
./setup_runners.sh company/frontend-app 2
```

#### 3. Directory Structure After Setup

```
~/action-runners/
├── setup_runners.sh
├── manage_repo.sh
├── manage_all.sh
├── list_repos.sh
├── health_check.sh
├── install_services.sh
├── viaduct/
│   ├── 1/
│   ├── 2/
│   ├── 3/
│   └── 4/
├── api-service/
│   ├── 1/
│   ├── 2/
│   ├── 3/
│   └── 4/
└── frontend-app/
    ├── 1/
    └── 2/
```

### Managing Your Runners

#### List All Repositories and Runner Status

```bash
./list_repos.sh
```

**Example output:**
```
GitHub Runner Repositories and Services:
========================================

Repository: viaduct
Runners:
  - Runner 1 (actions.runner.IgraLabs-viaduct.viaduct-runner-1.service): active
  - Runner 2 (actions.runner.IgraLabs-viaduct.viaduct-runner-2.service): active
  - Runner 3 (actions.runner.IgraLabs-viaduct.viaduct-runner-3.service): active
  - Runner 4 (actions.runner.IgraLabs-viaduct.viaduct-runner-4.service): active
  Total runners: 4

Repository: api-service
Runners:
  - Runner 1 (actions.runner.MyOrg-api-service.api-service-runner-1.service): active
  - Runner 2 (actions.runner.MyOrg-api-service.api-service-runner-2.service): active
  Total runners: 2
```

#### Repository-Specific Management

```bash
# Check status of all runners for 'viaduct' repository
./manage_repo.sh viaduct status

# Stop all runners for 'viaduct' repository
./manage_repo.sh viaduct stop

# Start all runners for 'viaduct' repository
./manage_repo.sh viaduct start

# Restart all runners for 'viaduct' repository
./manage_repo.sh viaduct restart

# View live logs for 'viaduct' runners (Ctrl+C to exit)
./manage_repo.sh viaduct logs

# Debug service discovery for 'viaduct'
./manage_repo.sh viaduct debug
```

#### Global Management (All Repositories)

```bash
# Check status of ALL runners across all repositories
./manage_all.sh status

# Stop ALL runners
./manage_all.sh stop

# Start ALL runners
./manage_all.sh start

# Restart ALL runners
./manage_all.sh restart

# View live logs from ALL runners
./manage_all.sh logs

# List all repositories (same as ./list_repos.sh)
./manage_all.sh list
```

#### Health Monitoring

```bash
# Run health check
./health_check.sh

# Set up automated health checks (run every 5 minutes)
crontab -e
# Add this line:
*/5 * * * * /home/$(whoami)/action-runners/health_check.sh >> /var/log/runner-health.log 2>&1
```

#### Install Runners as Services (If Not Done During Setup)

```bash
# Install all runners for a repository as services
./install_services.sh viaduct
```

## Using Runners in Your Workflows

### Basic Usage

Add this to your `.github/workflows/*.yml` files:

```yaml
jobs:
  build:
    runs-on: self-hosted
    steps:
      - uses: actions/checkout@v4
      - name: Build application
        run: make build
```

### Targeting Specific Repository Runners

```yaml
jobs:
  test:
    runs-on: [self-hosted, viaduct]  # Only use runners with 'viaduct' label
    steps:
      - uses: actions/checkout@v4
      - name: Run tests
        run: npm test
```

### Parallel Jobs Example

```yaml
jobs:
  test:
    strategy:
      matrix:
        shard: [1, 2, 3, 4]
    runs-on: self-hosted
    steps:
      - uses: actions/checkout@v4
      - name: Run test shard ${{ matrix.shard }}
        run: npm test -- --shard=${{ matrix.shard }}
```

### Resource Optimization in Workflows

```yaml
jobs:
  build:
    runs-on: self-hosted
    steps:
      - name: Configure maximum resource usage
        run: |
          # CPU cores detection
          if [ "$(uname)" == "Darwin" ]; then
            CORES=$(sysctl -n hw.ncpu)
          elif [ "$(expr substr $(uname -s) 1 5)" == "Linux" ]; then
            CORES=$(nproc)
          else
            CORES=4  # Default fallback
          fi

          # Set environment variables for this job
          echo "CORES=$CORES" >> $GITHUB_ENV
          echo "MAKEFLAGS=-j$CORES" >> $GITHUB_ENV
          echo "GRADLE_OPTS=-Dorg.gradle.parallel=true -Dorg.gradle.workers.max=$CORES" >> $GITHUB_ENV

      - uses: actions/checkout@v4
      - name: Build with all cores
        run: make build  # Will use MAKEFLAGS automatically
```

## Troubleshooting

### Common Issues

#### Runners Not Appearing in GitHub
- **Check token validity:** Tokens expire after 1 hour
- **Verify repository URL:** Ensure correct organization/repository format
- **Check network connectivity:** Runners need internet access

#### Services Not Found
```bash
# Debug service discovery
./manage_repo.sh viaduct debug

# Check if services exist
sudo systemctl list-units --type=service | grep actions.runner
```

#### Permission Denied Errors
```bash
# Fix permissions
sudo chown -R $USER:$USER ~/action-runners
chmod +x ~/action-runners/*.sh
```

#### Runners Offline After Reboot
```bash
# Restart all services
./manage_all.sh restart

# Or restart specific repository
./manage_repo.sh viaduct restart
```

### Log Analysis

#### View Detailed Logs
```bash
# Repository-specific logs
./manage_repo.sh viaduct logs

# All runners logs
./manage_all.sh logs

# Historical logs for a specific service
sudo journalctl -u actions.runner.IgraLabs-viaduct.viaduct-runner-1 --since "1 hour ago"
```

#### Log Cleanup
```bash
# Clean old logs (older than 2 weeks)
sudo journalctl --vacuum-time=2weeks

# Limit log size (keep only 1GB)
sudo journalctl --vacuum-size=1G
```

### Update Runners

```bash
# Create update script for all repositories
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

# Run the update
./update_all.sh
```

## Performance and Cost Optimization

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

Expected savings: **60-80%** reduction in CI/CD costs for most projects.

### Monitoring Resource Usage

```bash
# Monitor system resources
htop
df -h ~/action-runners/

# Check runner-specific resource usage
ps aux | grep Runner.Listener
```

This comprehensive setup gives you full control over your GitHub Actions infrastructure while significantly reducing costs and improving performance.

