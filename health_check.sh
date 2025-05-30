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
