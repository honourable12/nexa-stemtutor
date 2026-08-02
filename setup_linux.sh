#!/usr/bin/env bash

# setup_linux.sh
# Automatically detects Linux distribution and installs dependencies for Nexa Lab (Tauri v2 + llama-cpp)
# Designed for Ubuntu/Debian and Fedora/RPM systems.

set -euo pipefail

# Make sure script runs in the root folder of the project
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE"

echo "============================================="
echo "⚙️ Nexa Lab - Linux Setup & Dependency Installer"
echo "============================================="

# 1. Detect distribution
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS_NAME=$ID
    OS_LIKE=${ID_LIKE:-""}
else
    echo "⚠️ Cannot detect Linux distribution (/etc/os-release missing)."
    echo "Proceeding with manual checks..."
    OS_NAME="unknown"
    OS_LIKE="unknown"
fi

echo "Detected OS: ${NAME:-"Unknown Linux"} ($OS_NAME)"

# Function to check command existence
has_command() {
    command -v "$1" >/dev/null 2>&1
}

# 2. Install Packages based on OS
case "$OS_NAME" in
    ubuntu|debian|pop|mint|elementary)
        echo "Installing dependencies using apt-get..."
        sudo apt-get update
        sudo apt-get install -y \
            build-essential \
            curl \
            wget \
            pkg-config \
            libssl-dev \
            libgtk-3-dev \
            webkit2gtk-4.1-dev \
            libjavascriptcoregtk-4.1-dev \
            libsoup-3.0-dev \
            libayatana-appindicator3-dev \
            librsvg2-dev \
            cmake
        ;;
    fedora|nobara)
        echo "Installing dependencies using dnf..."
        sudo dnf groupinstall -y "Development Tools"
        sudo dnf install -y \
            curl \
            wget \
            pkgconf-pkg-config \
            openssl-devel \
            gtk3-devel \
            webkit2gtk4.1-devel \
            libsoup3-devel \
            libayatana-appindicator-devel \
            librsvg2-devel \
            cmake
        ;;
    *)
        # Check if it is Debian-like or RHEL-like as fallback
        if [[ "$OS_LIKE" == *"debian"* ]] || [[ "$OS_LIKE" == *"ubuntu"* ]]; then
            echo "Installing dependencies using apt-get (Debian-like fallback)..."
            sudo apt-get update
            sudo apt-get install -y build-essential curl wget pkg-config libssl-dev libgtk-3-dev webkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev libayatana-appindicator3-dev librsvg2-dev cmake
        elif [[ "$OS_LIKE" == *"rhel"* ]] || [[ "$OS_LIKE" == *"fedora"* ]] || [[ "$OS_LIKE" == *"centos"* ]]; then
            echo "Installing dependencies using dnf (Fedora-like fallback)..."
            sudo dnf groupinstall -y "Development Tools"
            sudo dnf install -y curl wget pkgconf-pkg-config openssl-devel gtk3-devel webkit2gtk4.1-devel libsoup3-devel libayatana-appindicator-devel librsvg2-devel cmake
        else
            echo "❌ Unsupported OS configuration."
            echo "Please manually install Tauri dependencies for your distribution."
            echo "Reference: https://v2.tauri.app/start/prerequisites/"
            exit 1
        fi
        ;;
esac

# 3. Check for Rust Toolchain
echo "Checking for Rust toolchain..."
if ! has_command cargo || ! has_command rustc; then
    echo "🦀 Rust toolchain not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # Source the cargo env for current script shell
    source "$HOME/.cargo/env" || true
    echo "Rust successfully installed!"
else
    echo "✓ Rust is already installed: $(rustc --version)"
fi

# 4. Check for Node.js
echo "Checking for Node.js..."
if ! has_command node || ! has_command npm; then
    echo "⚠️ Node.js and/or npm not found."
    echo "Please install Node.js (v18+) using your package manager."
    echo "Example (Ubuntu/Debian): sudo apt-get install -y nodejs npm"
    echo "Example (Fedora): sudo dnf install -y nodejs npm"
else
    echo "✓ Node.js is already installed: $(node --version)"
fi

# 5. Set executable permissions on scripts
echo "Configuring executable permissions..."
chmod +x "$HERE/download_model.sh"
if [ -f "$HERE/adtc-stem-tutor/setup.sh" ]; then
    chmod +x "$HERE/adtc-stem-tutor/setup.sh"
fi

echo "============================================="
echo "🎉 Setup complete! You are ready to run Nexa Lab."
echo "To get started:"
echo "  1. Run: bash download_model.sh"
echo "  2. Run: cd adtc-stem-tutor && npm install"
echo "  3. Start dev server: npm run tauri dev"
echo "============================================="
