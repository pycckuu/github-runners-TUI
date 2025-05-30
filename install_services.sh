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
