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
                service_name="actions.runner.$(whoami).${repo_name}-runner-${runner_num}"
                status=$(sudo systemctl is-active "$service_name" 2>/dev/null || echo "not-found")
                echo "  - Runner $runner_num: $status"
                ((runner_count++))
            fi
        done
        echo "  Total runners: $runner_count"
    fi
done
