#!/bin/bash
set -e

# Load common functions
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ ! -f "$SCRIPT_DIR/common.sh" ]; then
    echo "Error: common.sh not found in $SCRIPT_DIR" >&2
    exit 1
fi
source "$SCRIPT_DIR/common.sh"

# Auto-detect OS for platform-specific sudo handling
detect_os

cd "$HOME/action-runners" || {
    echo "Error: Cannot access $HOME/action-runners" >&2
    exit 1
}
for repo_dir in */; do
    if [ -d "$repo_dir" ] && [ "$repo_dir" != "*/" ]; then
        repo_name=${repo_dir%/}
        echo "Updating runners for $repo_name..."

        for runner_dir in "$repo_dir"*/; do
            if [ -d "$runner_dir" ] && [ -f "$runner_dir/config.sh" ]; then
                cd "$runner_dir"
                run_service_command "stop"
                ./bin/Runner.Listener update
                run_service_command "start"
                cd "$HOME/action-runners"
            fi
        done
    fi
done