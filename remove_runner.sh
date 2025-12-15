#!/bin/bash
set -e

# Load common functions
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ ! -f "$SCRIPT_DIR/common.sh" ]; then
    echo "Error: common.sh not found in $SCRIPT_DIR" >&2
    exit 1
fi
source "$SCRIPT_DIR/common.sh"

if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <repo-name> <runner-number>"
    echo "       $0 <repo-name> all"
    echo "Examples:"
    echo "  $0 viaduct 2        # Remove runner 2 from viaduct repo"
    echo "  $0 viaduct all      # Remove all runners from viaduct repo"
    exit 1
fi

REPO_NAME=$1
RUNNER_NUM=$2

# Validate inputs
validate_repo_name "$REPO_NAME" || exit 1
validate_runner_num "$RUNNER_NUM" || exit 1

# Auto-detect OS for platform-specific handling
detect_os

# Function to find service name for a specific runner (Linux only)
find_runner_service() {
    local repo=$1
    local num=$2
    if [ "$OS" == "linux" ]; then
        sudo systemctl list-units --all --type=service | grep "actions.runner.*${repo}-runner-${num}" | awk '{print $1}'
    fi
}

# Function to find all services for a repository (Linux only)
find_repo_services() {
    local repo=$1
    if [ "$OS" == "linux" ]; then
        sudo systemctl list-units --all --type=service | grep "actions.runner.*${repo}-runner-" | awk '{print $1}'
    fi
}

# Function to remove a single runner
remove_single_runner() {
    local repo=$1
    local num=$2
    local runner_dir="$HOME/action-runners/${repo}/${num}"

    echo "Removing runner ${num} for repository ${repo}..."

    # Check if runner directory exists
    if [ ! -d "$runner_dir" ]; then
        echo "❌ Runner directory not found: $runner_dir"
        return 1
    fi

    # Find and stop the service (platform-specific)
    if [ "$OS" == "linux" ]; then
        local service_name
        service_name=$(find_runner_service "$repo" "$num")
        if [ -n "$service_name" ]; then
            echo "  Stopping service: $service_name"
            sudo systemctl stop "$service_name" 2>/dev/null || echo "  ⚠️  Service stop failed or already stopped"

            echo "  Disabling service: $service_name"
            sudo systemctl disable "$service_name" 2>/dev/null || echo "  ⚠️  Service disable failed"
        else
            echo "  ⚠️  No systemd service found for runner $num"
        fi
    else
        echo "  Stopping service using svc.sh (macOS)..."
    fi

    # Try to uninstall using svc.sh if it exists
    cd "$runner_dir"
    if [ -f "svc.sh" ]; then
        echo "  Uninstalling service using svc.sh..."
        run_service_command "uninstall" 2>/dev/null || echo "  ⚠️  svc.sh uninstall failed"
    fi

    # Remove from GitHub (this will prompt for token)
    if [ -f "config.sh" ]; then
        echo "  Removing runner from GitHub..."
        echo "  (You may be prompted for a removal token from GitHub)"
        ./config.sh remove || echo "  ⚠️  GitHub removal failed - you may need to remove manually"
    fi

    # Ask for confirmation before deleting directory
    echo "  Do you want to delete the runner directory? (y/N)"
    read -r confirm
    if [[ "$confirm" == "y" || "$confirm" == "Y" ]]; then
        cd "$HOME/action-runners"
        rm -rf "${repo:?Repository name required}/${num:?Runner number required}"
        echo "  ✅ Runner directory deleted"
    else
        echo "  📁 Runner directory preserved at: $runner_dir"
    fi

    echo "  ✅ Runner ${num} removal completed"
}

# Main logic
if [ "$RUNNER_NUM" = "all" ]; then
    echo "Removing ALL runners for repository: $REPO_NAME"
    echo "Are you sure? This will remove all runners and their data. (y/N)"
    read -r confirm
    if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
        echo "Operation cancelled"
        exit 0
    fi

    # Find all runner directories
    for runner_dir in "$HOME/action-runners/$REPO_NAME"/*/; do
        if [ -d "$runner_dir" ]; then
            runner_num=$(basename "$runner_dir")
            remove_single_runner "$REPO_NAME" "$runner_num"
        fi
    done

    # Optionally remove the repository directory if empty
    if [ -z "$(ls -A "$HOME/action-runners/$REPO_NAME" 2>/dev/null)" ]; then
        echo "Repository directory is empty. Remove it? (y/N)"
        read -r confirm
        if [[ "$confirm" == "y" || "$confirm" == "Y" ]]; then
            rmdir "$HOME/action-runners/$REPO_NAME"
            echo "✅ Repository directory removed"
        fi
    fi
else
    remove_single_runner "$REPO_NAME" "$RUNNER_NUM"
fi

echo ""
echo "🔍 Checking remaining runners..."
./list_repos.sh
