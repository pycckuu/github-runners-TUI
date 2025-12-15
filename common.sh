#!/bin/bash
# Common functions shared across GitHub runner management scripts

# Detect OS and set the OS variable
# Sets: OS (either "linux" or "osx")
detect_os() {
    case "$(uname -s)" in
        Linux*)  OS="linux";;
        Darwin*) OS="osx";;
        *)       echo "Unsupported OS: $(uname -s)" >&2; exit 1;;
    esac
}

# Validate repository name contains only safe characters
# Usage: validate_repo_name <repo_name>
validate_repo_name() {
    local repo=$1
    if [ -z "$repo" ]; then
        echo "Error: Repository name cannot be empty" >&2
        return 1
    fi
    if ! [[ "$repo" =~ ^[a-zA-Z0-9_.-]+$ ]]; then
        echo "Error: Repository name contains invalid characters: $repo" >&2
        return 1
    fi
}

# Validate runner number (numeric or "all")
# Usage: validate_runner_num <runner_num>
validate_runner_num() {
    local num=$1
    if [ -z "$num" ]; then
        echo "Error: Runner number cannot be empty" >&2
        return 1
    fi
    if [ "$num" != "all" ] && ! [[ "$num" =~ ^[0-9]+$ ]]; then
        echo "Error: Runner number must be numeric or 'all': $num" >&2
        return 1
    fi
}

# Validate GitHub repository format (owner/repo)
# Usage: validate_github_repo <github_repo>
validate_github_repo() {
    local repo=$1
    if [ -z "$repo" ]; then
        echo "Error: GitHub repository cannot be empty" >&2
        return 1
    fi
    if ! [[ "$repo" =~ ^[a-zA-Z0-9_-]+/[a-zA-Z0-9_.-]+$ ]]; then
        echo "Error: Invalid GitHub repository format. Expected: owner/repo" >&2
        return 1
    fi
}

# Run a service command (install/start/stop/uninstall) with appropriate permissions
# Usage: run_service_command <command> [svc.sh path]
# Examples:
#   run_service_command "install"        # Uses ./svc.sh
#   run_service_command "start" "$runner_dir/svc.sh"
run_service_command() {
    local cmd=$1
    local svc_script="${2:-./svc.sh}"

    if [ ! -f "$svc_script" ]; then
        echo "Error: svc.sh not found at $svc_script (cwd: $PWD)" >&2
        return 1
    fi

    if [ "$OS" == "linux" ]; then
        sudo "$svc_script" "$cmd"
    else
        "$svc_script" "$cmd"
    fi
}
