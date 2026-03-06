#!/bin/bash

set -e

echo "================================"
echo "  convx Installation Script"
echo "================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if running on Linux
if [[ "$OSTYPE" != "linux-gnu"* ]]; then
    echo -e "${YELLOW}Warning: This script is designed for Linux. For other systems, please install dependencies manually.${NC}"
    echo ""
fi

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

echo "Step 1: Checking system dependencies..."
echo ""

# Check for FFmpeg
if command_exists ffmpeg; then
    FFMPEG_VERSION=$(ffmpeg -version 2>&1 | head -n1)
    echo -e "${GREEN}✓${NC} FFmpeg is installed: $FFMPEG_VERSION"
else
    echo -e "${RED}✗${NC} FFmpeg is not installed"
    NEED_FFMPEG=1
fi

# Check for libvips
if command_exists vips; then
    VIPS_VERSION=$(vips --version 2>&1)
    echo -e "${GREEN}✓${NC} libvips is installed: $VIPS_VERSION"
else
    echo -e "${RED}✗${NC} libvips is not installed"
    NEED_VIPS=1
fi

# Check for Rust/Cargo
if command_exists cargo; then
    RUST_VERSION=$(rustc --version 2>&1)
    echo -e "${GREEN}✓${NC} Rust is installed: $RUST_VERSION"
else
    echo -e "${RED}✗${NC} Rust is not installed"
    NEED_RUST=1
fi

echo ""

# Install missing dependencies
if [[ -n "$NEED_FFMPEG" || -n "$NEED_VIPS" ]]; then
    echo "Step 2: Installing missing dependencies..."
    echo ""
    
    # Detect package manager
    if command_exists apt-get; then
        PKG_MANAGER="apt-get"
        INSTALL_CMD="sudo apt-get install -y"
    elif command_exists dnf; then
        PKG_MANAGER="dnf"
        INSTALL_CMD="sudo dnf install -y"
    elif command_exists yum; then
        PKG_MANAGER="yum"
        INSTALL_CMD="sudo yum install -y"
    elif command_exists pacman; then
        PKG_MANAGER="pacman"
        INSTALL_CMD="sudo pacman -S --noconfirm"
    elif command_exists brew; then
        PKG_MANAGER="brew"
        INSTALL_CMD="brew install"
    else
        echo -e "${RED}Error: No supported package manager found.${NC}"
        echo "Please install FFmpeg and libvips manually:"
        echo "  - FFmpeg: https://ffmpeg.org/download.html"
        echo "  - libvips: https://www.libvips.org/install.html"
        exit 1
    fi
    
    echo "Using package manager: $PKG_MANAGER"
    echo ""
    
    if [[ -n "$NEED_FFMPEG" ]]; then
        echo "Installing FFmpeg..."
        if [[ "$PKG_MANAGER" == "brew" ]]; then
            $INSTALL_CMD ffmpeg
        elif [[ "$PKG_MANAGER" == "pacman" ]]; then
            $INSTALL_CMD ffmpeg
        else
            $INSTALL_CMD ffmpeg
        fi
        echo -e "${GREEN}✓${NC} FFmpeg installed"
        echo ""
    fi
    
    if [[ -n "$NEED_VIPS" ]]; then
        echo "Installing libvips..."
        if [[ "$PKG_MANAGER" == "apt-get" ]]; then
            $INSTALL_CMD libvips-tools
        elif [[ "$PKG_MANAGER" == "brew" ]]; then
            $INSTALL_CMD vips
        elif [[ "$PKG_MANAGER" == "pacman" ]]; then
            $INSTALL_CMD libvips
        elif [[ "$PKG_MANAGER" == "dnf" || "$PKG_MANAGER" == "yum" ]]; then
            $INSTALL_CMD vips vips-tools
        fi
        echo -e "${GREEN}✓${NC} libvips installed"
        echo ""
    fi
else
    echo "Step 2: All system dependencies are already installed!"
    echo ""
fi

# Install Rust if needed
if [[ -n "$NEED_RUST" ]]; then
    echo "Step 3: Installing Rust..."
    echo ""
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}✓${NC} Rust installed"
    echo ""
else
    echo "Step 3: Rust is already installed!"
    echo ""
fi

# Build convx
echo "Step 4: Building convx..."
echo ""
cd convx-core
if [[ -n "$NEED_RUST" ]]; then
    source "$HOME/.cargo/env"
fi
cargo build --release
echo ""
echo -e "${GREEN}✓${NC} convx built successfully!"
echo ""

# Create symlink
echo "Step 5: Creating symlink..."
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

if [[ -L "$INSTALL_DIR/convx" || -f "$INSTALL_DIR/convx" ]]; then
    rm "$INSTALL_DIR/convx"
fi

ln -s "$(pwd)/target/release/convx" "$INSTALL_DIR/convx"
echo -e "${GREEN}✓${NC} Created symlink: $INSTALL_DIR/convx"
echo ""

# Check if ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo -e "${YELLOW}Warning: $HOME/.local/bin is not in your PATH${NC}"
    echo "Add this line to your ~/.bashrc or ~/.zshrc:"
    echo ""
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
fi

echo "================================"
echo -e "${GREEN}Installation complete!${NC}"
echo "================================"
echo ""
echo "Run 'convx --help' to get started"
echo "Run 'convx formats' to see supported formats"
echo ""
echo "Example usage:"
echo "  convx convert input.png --to webp"
echo "  convx convert video.mp4 --to gif --fps 10"
echo "  convx convert audio.wav --to mp3"
echo ""
