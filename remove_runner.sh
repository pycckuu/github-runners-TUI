#!/bin/bash
set -euo pipefail

# Load common functions
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ ! -f "$SCRIPT_DIR/common.sh" ]; then
    echo "Error: common.sh not found in $SCRIPT_DIR" >&2
    exit 1
fi
source "$SCRIPT_DIR/common.sh"

# Constants
RUNNERS_BASE_DIR="$HOME/action-runners"

# =============================================================================
# Output Helpers
# =============================================================================

# Print an indented message (for sub-steps)
log_info() {
    echo "  $1"
}

# Print an indented error message to stderr
log_error() {
    echo "  $1" >&2
}

# Print a top-level status message
log_status() {
    echo "$1"
}

# =============================================================================
# User Interaction Helpers
# =============================================================================

# Ask for yes/no confirmation
# Usage: confirm_action "Question?" && do_something
confirm_action() {
    local prompt="$1"
    local response
    echo "$prompt (y/N)"
    read -r response
    [[ "$response" == "y" || "$response" == "Y" ]]
}

# Prompt for GitHub repo if not already set
# Sets GITHUB_REPO variable if user provides input
# Returns: 0 if repo is set, 1 if skipped
prompt_github_repo() {
    if [ -n "$GITHUB_REPO" ]; then
        return 0
    fi

    log_info "GitHub repository not provided."
    log_info "Enter the full repository path (e.g., IgraLabs/viaduct):"
    read -r GITHUB_REPO

    if [ -z "$GITHUB_REPO" ]; then
        log_error "No repository provided, skipping API removal"
        return 1
    fi

    validate_github_repo "$GITHUB_REPO" || return 1
    return 0
}

# =============================================================================
# Usage and Argument Parsing
# =============================================================================

usage() {
    echo "Usage: $0 <repo-name> <runner-number> [--github-repo OWNER/REPO]"
    echo "       $0 <repo-name> all [--github-repo OWNER/REPO]"
    echo ""
    echo "Options:"
    echo "  --github-repo OWNER/REPO  GitHub repository (e.g., IgraLabs/viaduct)"
    echo "                            Used for API fallback when local removal fails"
    echo ""
    echo "Examples:"
    echo "  $0 viaduct 2                              # Remove runner 2"
    echo "  $0 viaduct all                            # Remove all runners"
    echo "  $0 viaduct all --github-repo IgraLabs/viaduct  # With explicit repo"
    exit 1
}

parse_arguments() {
    if [ "$#" -lt 2 ]; then
        usage
    fi

    REPO_NAME=$1
    RUNNER_NUM=$2
    GITHUB_REPO=""

    shift 2
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --github-repo)
                if [ "$#" -lt 2 ] || [ -z "$2" ]; then
                    echo "Error: --github-repo requires a value" >&2
                    exit 1
                fi
                GITHUB_REPO="$2"
                shift 2
                ;;
            -h|--help)
                usage
                ;;
            *)
                echo "Error: Unknown option: $1" >&2
                usage
                ;;
        esac
    done
}

validate_arguments() {
    validate_repo_name "$REPO_NAME" || exit 1
    validate_runner_num "$RUNNER_NUM" || exit 1
    if [ -n "$GITHUB_REPO" ]; then
        validate_github_repo "$GITHUB_REPO" || exit 1
    fi
}

# =============================================================================
# Service Management (Linux-specific)
# =============================================================================

# Find systemd service name for a specific runner
# Usage: find_runner_service <repo> <runner_num>
find_runner_service() {
    local repo=$1
    local num=$2

    if [ "$OS" != "linux" ]; then
        return
    fi

    sudo systemctl list-units --all --type=service \
        | grep -F "actions.runner" \
        | grep -F "${repo}-runner-${num}" \
        | awk '{print $1}' || true
}

# Stop and disable the runner's systemd service
# Usage: stop_runner_service <repo> <runner_num>
stop_runner_service() {
    local repo=$1
    local num=$2

    if [ "$OS" != "linux" ]; then
        log_info "macOS: Service will be stopped during uninstall step"
        return
    fi

    local service_name
    service_name=$(find_runner_service "$repo" "$num")

    if [ -z "$service_name" ]; then
        log_info "No systemd service found for runner $num"
        return
    fi

    log_info "Stopping service: $service_name"
    sudo systemctl stop "$service_name" 2>/dev/null || log_info "Service stop failed or already stopped"

    log_info "Disabling service: $service_name"
    sudo systemctl disable "$service_name" 2>/dev/null || log_info "Service disable failed"
}

# =============================================================================
# GitHub API Removal
# =============================================================================

# Remove runner via GitHub CLI API
# Usage: remove_runner_via_api <github_repo> <runner_name>
# Returns: 0 on success, 1 on failure
remove_runner_via_api() {
    local github_repo=$1
    local runner_name=$2

    if ! command -v github &> /dev/null; then
        log_error "GitHub CLI (github) not found"
        return 1
    fi

    local runner_id
    runner_id=$(github api "repos/${github_repo}/actions/runners" \
        --jq ".runners[] | select(.name==\"${runner_name}\") | .id" 2>/dev/null) || true

    if [ -z "$runner_id" ]; then
        log_error "Runner '$runner_name' not found on GitHub"
        return 1
    fi

    log_info "Removing via GitHub API (ID: $runner_id)..."
    if github api -X DELETE "repos/${github_repo}/actions/runners/${runner_id}" 2>/dev/null; then
        log_info "API removal successful"
        return 0
    else
        log_error "API removal failed"
        return 1
    fi
}

# Show manual removal instructions when automated removal fails
show_manual_removal_instructions() {
    echo ""
    log_info "Could not remove runner from GitHub automatically."
    log_info "Please remove manually at:"
    if [ -n "$GITHUB_REPO" ]; then
        log_info "  https://github.com/${GITHUB_REPO}/settings/actions/runners"
    else
        log_info "  https://github.com/<owner>/<repo>/settings/actions/runners"
    fi
    echo ""
}

# =============================================================================
# Runner Removal Logic
# =============================================================================

# Uninstall runner service using svc.sh
# Usage: uninstall_runner_service <runner_dir>
uninstall_runner_service() {
    local runner_dir=$1

    (
        cd "$runner_dir" || { log_error "Failed to cd to $runner_dir"; exit 1; }
        if [ -f "svc.sh" ]; then
            log_info "Uninstalling service using svc.sh..."
            run_service_command "uninstall" 2>/dev/null || log_info "svc.sh uninstall failed"
        fi
    )
}

# Remove runner from GitHub using config.sh
# Usage: remove_runner_via_config <runner_dir>
# Returns: 0 on success, 1 on failure
remove_runner_via_config() {
    local runner_dir=$1

    (
        cd "$runner_dir" || { log_error "Failed to cd to $runner_dir"; return 1; }
        if [ ! -f "config.sh" ]; then
            exit 1
        fi

        log_info "Removing runner from GitHub via config.sh..."
        log_info "(You may be prompted for a removal token from GitHub)"
        ./config.sh remove
    )
}

# Attempt to remove runner from GitHub (tries config.sh first, then API)
# Usage: remove_runner_from_github <runner_dir> <runner_name> <dir_exists>
# Returns: 0 if removal succeeded via either method, 1 if both failed
remove_runner_from_github() {
    local runner_dir=$1
    local runner_name=$2
    local dir_exists=$3

    # Try config.sh removal first if directory exists
    if [ "$dir_exists" = true ]; then
        if remove_runner_via_config "$runner_dir"; then
            return 0
        fi
        log_info "config.sh removal failed, will try API fallback..."
    fi

    # Fall back to API removal
    log_info "Attempting GitHub API removal..."
    if prompt_github_repo && remove_runner_via_api "$GITHUB_REPO" "$runner_name"; then
        return 0
    fi

    return 1
}

# Delete the runner directory after confirmation
# Usage: delete_runner_directory <runner_dir> <repo> <num>
delete_runner_directory() {
    local runner_dir=$1
    local repo=$2
    local num=$3

    if confirm_action "  Do you want to delete the runner directory?"; then
        rm -rf "${RUNNERS_BASE_DIR:?Base directory not set}/${repo:?Repository name required}/${num:?Runner number required}"
        log_info "Runner directory deleted"
    else
        log_info "Runner directory preserved at: $runner_dir"
    fi
}

# Remove a single runner (main removal function)
# Usage: remove_single_runner <repo> <runner_num>
remove_single_runner() {
    local repo=$1
    local num=$2
    local runner_dir="$RUNNERS_BASE_DIR/${repo}/${num}"
    local runner_name="${repo}-runner-${num}"
    local dir_exists=true

    log_status "Removing runner ${num} for repository ${repo}..."

    # Check if runner directory exists
    if [ ! -d "$runner_dir" ]; then
        log_info "Runner directory not found: $runner_dir"
        log_info "Will attempt removal via GitHub API..."
        dir_exists=false
    fi

    # Stop and uninstall service if directory exists
    if [ "$dir_exists" = true ]; then
        stop_runner_service "$repo" "$num"
        uninstall_runner_service "$runner_dir"
    fi

    # Attempt to remove from GitHub
    if ! remove_runner_from_github "$runner_dir" "$runner_name" "$dir_exists"; then
        show_manual_removal_instructions
    fi

    # Optionally delete runner directory
    if [ "$dir_exists" = true ]; then
        delete_runner_directory "$runner_dir" "$repo" "$num"
    fi

    echo "  Done: Runner ${num} removal completed"
}

# Remove all runners for a repository
# Usage: remove_all_runners <repo>
remove_all_runners() {
    local repo=$1
    local repo_dir="$RUNNERS_BASE_DIR/$repo"

    log_status "Removing ALL runners for repository: $repo"

    if [ ! -d "$repo_dir" ]; then
        log_error "Repository directory not found: $repo_dir"
        log_info "No local runners to remove. Use --github-repo to remove via API."
        return 1
    fi

    if ! confirm_action "Are you sure? This will remove all runners and their data."; then
        log_status "Operation cancelled"
        exit 0
    fi

    # Find and remove all runner directories
    shopt -s nullglob
    local runner_dirs=("$repo_dir"/*/)
    shopt -u nullglob

    if [ ${#runner_dirs[@]} -eq 0 ]; then
        log_info "No runner directories found in $repo_dir"
        return 0
    fi

    for runner_dir in "${runner_dirs[@]}"; do
        local runner_num
        runner_num=$(basename "$runner_dir")
        remove_single_runner "$repo" "$runner_num"
    done

    # Optionally remove empty repository directory
    if [ -z "$(ls -A "$repo_dir" 2>/dev/null)" ]; then
        if confirm_action "Repository directory is empty. Remove it?"; then
            rmdir "$repo_dir"
            log_status "Done: Repository directory removed"
        fi
    fi
}

# =============================================================================
# Main
# =============================================================================

main() {
    parse_arguments "$@"
    validate_arguments
    detect_os

    if [ "$RUNNER_NUM" = "all" ]; then
        remove_all_runners "$REPO_NAME"
    else
        remove_single_runner "$REPO_NAME" "$RUNNER_NUM"
    fi

    echo ""
    echo "Checking remaining runners..."
    "$SCRIPT_DIR/list_repos.sh"
}

main "$@"
