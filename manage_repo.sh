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


