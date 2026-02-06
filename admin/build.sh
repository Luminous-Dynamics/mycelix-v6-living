#!/bin/bash
# Build script for Mycelix Admin Panel
# This script builds the React admin panel for embedding in the Rust binary.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Building Mycelix Admin Panel ==="

# Check for Node.js
if ! command -v node &> /dev/null; then
    echo "Error: Node.js is required but not installed."
    echo "Please install Node.js 18+ from https://nodejs.org/"
    exit 1
fi

# Check for npm
if ! command -v npm &> /dev/null; then
    echo "Error: npm is required but not installed."
    exit 1
fi

NODE_VERSION=$(node -v | cut -d'.' -f1 | sed 's/v//')
if [ "$NODE_VERSION" -lt 18 ]; then
    echo "Warning: Node.js 18+ is recommended. You have Node.js v$(node -v)"
fi

# Install dependencies
echo "Installing dependencies..."
npm install

# Build the project
echo "Building production bundle..."
npm run build

# Check build output
if [ -d "dist" ]; then
    echo ""
    echo "=== Build successful ==="
    echo "Output directory: $SCRIPT_DIR/dist"
    echo ""
    echo "Files:"
    ls -la dist/
    echo ""
    echo "To use the built admin panel:"
    echo "  1. Copy dist/ to a location accessible by the server"
    echo "  2. Or use rust-embed to embed assets in the binary"
else
    echo "Error: Build failed - dist directory not created"
    exit 1
fi
