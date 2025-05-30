#!/bin/bash
set -e

echo "Setting up build environment for GitHub self-hosted runners..."

# Install essential build tools
echo "Installing system build tools..."
sudo apt update
sudo apt install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libclang-dev \
    llvm-dev \
    libudev-dev \
    libc6-dev \
    curl \
    git \
    ca-certificates

# Install Rust toolchains to prevent CI/local mismatches and concurrent conflicts
echo "Installing Rust toolchains..."

# Check if running as root (shouldn't be for self-hosted runners)
if [ "$EUID" -eq 0 ]; then
    echo "❌ Error: This script should not be run as root for proper Rust installation"
    echo "   Self-hosted runners should run as a regular user"
    exit 1
fi

# Install rustup if not present
if ! command -v rustup &> /dev/null; then
    echo "Installing rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none
    source ~/.cargo/env
else
    echo "rustup already installed, updating..."
    rustup self update
fi

# Ensure cargo environment is available
source ~/.cargo/env

# Install stable toolchain and set as default
echo "Installing stable toolchain..."
rustup toolchain install stable --profile default
rustup default stable

# Install nightly toolchain (for miri and other nightly-only tools)
echo "Installing nightly toolchain..."
rustup toolchain install nightly --profile default

# Verify installations
echo "Verifying Rust installations..."
echo "Stable toolchain:"
rustup run stable rustc --version
rustup run stable cargo --version

echo "Nightly toolchain:"
rustup run nightly rustc --version
rustup run nightly cargo --version

# Set up environment variables for libclang and other system dependencies
echo "Configuring system environment..."
LIBCLANG_PATH=$(find /usr/lib* -name "libclang.so*" -o -name "libclang-*.so*" | head -1 | xargs dirname 2>/dev/null)
if [ -n "$LIBCLANG_PATH" ]; then
    echo "Found libclang at: $LIBCLANG_PATH"
    sudo tee -a /etc/environment > /dev/null << EOF
LIBCLANG_PATH=$LIBCLANG_PATH
EOF
else
    echo "⚠️  Warning: libclang not found, some crates may fail to build"
fi

# Add cargo bin to system PATH for all users
echo "Configuring system-wide PATH..."
sudo tee /etc/environment > /dev/null << 'EOF'
PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/games:/usr/local/games:/home/runner/.cargo/bin"
EOF

# Append LIBCLANG_PATH if found
if [ -n "$LIBCLANG_PATH" ]; then
    echo "LIBCLANG_PATH=$LIBCLANG_PATH" | sudo tee -a /etc/environment > /dev/null
fi

# Create cargo config to prevent concurrent installation conflicts
echo "Creating cargo config for CI stability..."
mkdir -p ~/.cargo
cat > ~/.cargo/config.toml << 'EOF'
[build]
# Use separate target directories per job to prevent conflicts
# This can be overridden in workflows with CARGO_TARGET_DIR if needed

[net]
# Increase retry count for network issues
retry = 3

[registry]
default = "crates-io"
EOF

# Set up environment variables for GitHub Actions
echo "Setting up GitHub Actions environment..."
mkdir -p ~/actions-runner
cat > ~/actions-runner/.env << EOF
# Rust environment
CARGO_HOME=/home/runner/.cargo
RUSTUP_HOME=/home/runner/.rustup
PATH=/home/runner/.cargo/bin:\$PATH

# System dependencies for bindgen and similar crates
${LIBCLANG_PATH:+LIBCLANG_PATH=$LIBCLANG_PATH}

# Prevent cargo from using network during build (use vendored deps)
# CARGO_NET_OFFLINE=true  # Uncomment if using vendored dependencies

# Use job-specific target directories to prevent concurrent conflicts
# CARGO_TARGET_DIR=/tmp/cargo-target-\${GITHUB_RUN_ID}-\${GITHUB_JOB}
EOF

echo ""
echo "✅ Enhanced build environment setup complete!"
echo ""
echo "📋 Summary of changes:"
echo "   • Installed system build tools including libclang"
echo "   • Installed stable Rust toolchain (set as default)"
echo "   • Installed nightly Rust toolchain (for miri jobs)"
echo "   • Configured system PATH and environment variables"
echo "   • Created cargo config for CI stability"
echo "   • Set up GitHub Actions environment variables"
echo ""
echo "🔧 Usage in workflows:"
echo "   • Use 'cargo +stable' for stable builds"
echo "   • Use 'cargo +nightly' for nightly builds (miri, etc.)"
echo "   • Source environment: 'source ~/.cargo/env'"
echo "   • No need for rust-toolchain actions on self-hosted runners"
echo ""
echo "💡 To verify setup:"
echo "   rustup show"
echo "   cargo +stable --version"
echo "   cargo +nightly --version"
${LIBCLANG_PATH:+echo "   LIBCLANG_PATH=$LIBCLANG_PATH"}