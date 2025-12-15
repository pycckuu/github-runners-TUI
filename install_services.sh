#!/bin/bash
set -e

# Load common functions
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ ! -f "$SCRIPT_DIR/common.sh" ]; then
    echo "Error: common.sh not found in $SCRIPT_DIR" >&2
    exit 1
fi
source "$SCRIPT_DIR/common.sh"

if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <repo-name>"
    echo "Example: $0 viaduct"
    exit 1
fi

REPO_NAME=$1

# Validate repo name
validate_repo_name "$REPO_NAME" || exit 1

# Auto-detect OS for platform-specific sudo handling
detect_os

echo "Installing services for repository: $REPO_NAME (platform: $OS)"

cd "$HOME/action-runners/$REPO_NAME" || {
    echo "Error: Cannot access directory $HOME/action-runners/$REPO_NAME" >&2
    exit 1
}

for runner_dir in */; do
    if [ -d "$runner_dir" ] && [ -f "$runner_dir/svc.sh" ]; then
        runner_num=${runner_dir%/}
        echo "Installing service for runner $runner_num..."

        cd "$runner_dir"
        run_service_command "install"
        run_service_command "start"
        cd ..

        echo "✅ Runner $runner_num installed and started as service"
    fi
done

echo "All services installed for repository: $REPO_NAME"
