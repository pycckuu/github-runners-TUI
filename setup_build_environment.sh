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
    curl \
    git \
    ca-certificates

echo "✅ Build environment setup complete!"