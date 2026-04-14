#!/usr/bin/env bash
set -euo pipefail

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
echo "Building repo39 v${VERSION}"

mkdir -p dist

# macOS ARM64 (Apple Silicon)
echo "→ aarch64-apple-darwin"
cargo build --release --target aarch64-apple-darwin
cp target/aarch64-apple-darwin/release/repo39-cli dist/repo39-cli-macos-arm64
cp target/aarch64-apple-darwin/release/repo39-mcp dist/repo39-mcp-macos-arm64

# macOS x86_64
echo "→ x86_64-apple-darwin"
cargo build --release --target x86_64-apple-darwin
cp target/x86_64-apple-darwin/release/repo39-cli dist/repo39-cli-macos-x64
cp target/x86_64-apple-darwin/release/repo39-mcp dist/repo39-mcp-macos-x64

# Linux ARM64 (via zigbuild)
echo "→ aarch64-unknown-linux-gnu"
cargo zigbuild --release --target aarch64-unknown-linux-gnu
cp target/aarch64-unknown-linux-gnu/release/repo39-cli dist/repo39-cli-linux-arm64
cp target/aarch64-unknown-linux-gnu/release/repo39-mcp dist/repo39-mcp-linux-arm64

# Linux x86_64 (via zigbuild)
echo "→ x86_64-unknown-linux-gnu"
cargo zigbuild --release --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/repo39-cli dist/repo39-cli-linux-x64
cp target/x86_64-unknown-linux-gnu/release/repo39-mcp dist/repo39-mcp-linux-x64

echo "Done. Binaries in dist/"
ls -lh dist/
