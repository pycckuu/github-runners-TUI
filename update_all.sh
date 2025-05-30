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