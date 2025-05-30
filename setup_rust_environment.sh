#!/bin/bash
set -e

echo "Setting up Rust environment for self-hosted runners..."

# Remove any broken Rust installation
echo "Cleaning up any existing broken Rust installation..."
rm -rf ~/.cargo ~/.rustup 2>/dev/null || true

# Install Rust
echo "Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable

# Source Rust environment
echo "Sourcing Rust environment..."
source ~/.cargo/env

# Verify installation
echo "Verifying Rust installation..."
if ! command -v rustup &> /dev/null; then
    echo "❌ Error: rustup not found after installation"
    exit 1
fi

echo "✅ Rust installation successful!"
echo "  rustup: $(rustup --version)"
echo "  rustc: $(rustc --version)"
echo "  cargo: $(cargo --version)"

# Set stable as default
rustup default stable

# Install essential components
echo "Installing Rust components..."
rustup component add rustfmt
rustup component add clippy

# Install nightly for advanced tools
echo "Installing nightly toolchain..."
rustup toolchain install nightly
rustup component add rustfmt --toolchain nightly
rustup component add clippy --toolchain nightly
rustup component add miri --toolchain nightly

# Install common cargo tools
echo "Installing cargo tools..."
cargo install cargo-audit || echo "Warning: cargo-audit installation failed"
cargo install typos-cli || echo "Warning: typos-cli installation failed"

# Add Rust to shell profile
if ! grep -q ".cargo/bin" ~/.bashrc; then
    echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
    echo "Added Rust to ~/.bashrc"
fi

echo "✅ Rust environment setup complete!"
echo ""
echo "Verification:"
echo "  Default toolchain: $(rustup default)"
echo "  Installed components: $(rustup component list --installed | tr '\n' ' ')"