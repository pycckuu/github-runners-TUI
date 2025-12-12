#!/bin/bash
set -e

# Check if required arguments are provided
if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <github-repo> [num-runners]"
    echo "Example: $0 myorg/myrepo 4"
    echo "Note: If number of runners is not provided, 4 will be used as default."
    exit 1
fi

# Configuration variables
GITHUB_REPO=$1
NUM_RUNNERS=${2:-4}  # Use the second argument or default to 4 runners
REPO_NAME=$(echo "$GITHUB_REPO" | cut -d'/' -f2)  # Extract repo name from org/repo
REPO_URL="https://github.com/${GITHUB_REPO}"
BASE_DIR="$HOME/action-runners/${REPO_NAME}"
RUNNER_VERSION="2.330.0"

# Auto-detect OS (GitHub uses 'osx' for macOS)
case "$(uname -s)" in
    Linux*)  OS="linux";;
    Darwin*) OS="osx";;
    *)       echo "Unsupported OS: $(uname -s)"; exit 1;;
esac

# Auto-detect architecture
case "$(uname -m)" in
    x86_64)  ARCHITECTURE="x64";;
    aarch64) ARCHITECTURE="arm64";;
    arm64)   ARCHITECTURE="arm64";;
    *)       echo "Unsupported architecture: $(uname -m)"; exit 1;;
esac

echo "Detected platform: ${OS}-${ARCHITECTURE}"

echo "Setting up ${NUM_RUNNERS} runners for repository: ${GITHUB_REPO}"
echo "Using base directory: ${BASE_DIR}"

# Function to set up a runner
setup_runner() {
    local runner_number=$1
    local runner_dir="${BASE_DIR}/${runner_number}"
    local runner_name="${REPO_NAME}-runner-${runner_number}"
    
    echo "Setting up runner ${runner_number} in ${runner_dir}..."
    
    # Create directory if it doesn't exist
    mkdir -p "${runner_dir}"
    
    # Enter the runner directory
    cd "${runner_dir}"
    
    # Download the runner if not already present
    if [ ! -f "actions-runner-${OS}-${ARCHITECTURE}-${RUNNER_VERSION}.tar.gz" ]; then
        echo "Downloading runner package..."
        # Remove old binaries to ensure clean extraction
        rm -rf bin externals ./*.sh 2>/dev/null || true
        curl -o "actions-runner-${OS}-${ARCHITECTURE}-${RUNNER_VERSION}.tar.gz" -L "https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-${OS}-${ARCHITECTURE}-${RUNNER_VERSION}.tar.gz"
        echo "Extracting runner package..."
        tar xzf "./actions-runner-${OS}-${ARCHITECTURE}-${RUNNER_VERSION}.tar.gz"
    elif [ ! -f "./config.sh" ]; then
        # Tarball exists but not extracted - clean before extraction
        rm -rf bin externals ./*.sh 2>/dev/null || true
        echo "Extracting runner package..."
        tar xzf "./actions-runner-${OS}-${ARCHITECTURE}-${RUNNER_VERSION}.tar.gz"
    fi
    
    # Check for existing runner configuration
    if [ -f "./.runner" ]; then
        echo "Runner ${runner_number} is already configured. Skipping configuration."
        return 0
    fi
    
    # Prompt for token if not already provided
    if [ -z "$TOKEN" ]; then
        echo "Please enter your GitHub runner registration token:"
        read -r TOKEN
    fi
    
    # Configure the runner (--replace allows re-registering existing runner names)
    echo "Configuring runner ${runner_number}..."
    ./config.sh --url "$REPO_URL" --token "$TOKEN" --name "$runner_name" \
                --labels "self-hosted,${OS},${ARCHITECTURE},${REPO_NAME}" --work "_work" --unattended --replace
    
    # Set up concurrent job processing (optional)
    echo '{"workJobConcurrency":"2"}' > .runner.jitconfig
    
    # Install and start as service (works on both Linux/systemd and macOS/launchd)
    echo "Do you want to install runner ${runner_number} as a service? (y/n)"
    read -r install_service

    if [[ "$install_service" == "y" || "$install_service" == "Y" ]]; then
        ./svc.sh install
        ./svc.sh start
        echo "Runner ${runner_number} installed and started as a service."
    else
        echo "To start the runner manually, use: cd ${runner_dir} && ./run.sh"
    fi
}

# Main script
echo "Setting up ${NUM_RUNNERS} GitHub runners for repository..."

# Create directories if they don't exist
for i in $(seq 1 "$NUM_RUNNERS"); do
    mkdir -p "${BASE_DIR}/${i}"
done

# Prompt for GitHub token
echo "Please enter your GitHub runner registration token:"
read -r TOKEN

# Set up each runner
for i in $(seq 1 "$NUM_RUNNERS"); do
    setup_runner "$i"
done

echo "All ${NUM_RUNNERS} runners have been set up!"
echo "You can verify they are connected in your GitHub repository settings under Actions > Runners."
echo ""
echo "To manage your runners:"
echo "- View status: Visit your repository's Settings > Actions > Runners page"
echo "- Update runners: cd into each runner directory and run ./bin/Runner.Listener update"
echo "- Remove runners: cd into each runner directory and run ./config.sh remove --token YOUR_REMOVAL_TOKEN"

# Provide instructions for workflow usage
echo ""
echo "To use these runners in your workflows, add this to your .github/workflows/*.yml files:"
echo "jobs:"
echo "  your_job_name:"
echo "    runs-on: self-hosted"
echo "    # Or specifically target your runners with:"
echo "    # runs-on: [self-hosted, ${REPO_NAME}]"

