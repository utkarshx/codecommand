#!/bin/bash

set -e  # Exit on any error

echo "🧹 Cleaning previous builds..."
rm -rf npx-cli/dist
mkdir -p npx-cli/dist/macos-arm64

echo "🔨 Building frontend..."
npm run frontend:build

echo "🔨 Building Rust binaries..."
cargo build --release --manifest-path backend/Cargo.toml
cargo build --release --bin mcp_task_server --manifest-path backend/Cargo.toml

echo "📦 Creating distribution package..."

# Copy the main binary
cp target/release/codecommand codecommand
cp target/release/mcp_task_server codecommand-mcp

zip codecommand.zip codecommand
zip codecommand-mcp.zip codecommand-mcp

rm codecommand codecommand-mcp

mv codecommand.zip npx-cli/dist/macos-arm64/codecommand.zip
mv codecommand-mcp.zip npx-cli/dist/macos-arm64/codecommand-mcp.zip

echo "✅ NPM package ready!"
echo "📁 Files created:"
echo "   - npx-cli/dist/macos-arm64/codecommand.zip"
echo "   - npx-cli/dist/macos-arm64/codecommand-mcp.zip"