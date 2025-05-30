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


