#!/usr/bin/env bash
set -euo pipefail

# Static musl linking opens many file descriptors at once; raise the limit.
ulimit -n 4096 2>/dev/null || true

VERSION=$(cargo metadata --no-deps --format-version 1 | python3 -c "import sys,json; print(json.load(sys.stdin)['packages'][0]['version'])")
NAME="repo39"
OUT_DIR="dist"

mkdir -p "$OUT_DIR"

echo "=== Building $NAME v$VERSION ==="

# macOS ARM64 (native)
echo ""
echo "--- macOS arm64 (aarch64-apple-darwin) ---"
cargo build --release --target aarch64-apple-darwin
cp target/aarch64-apple-darwin/release/$NAME "$OUT_DIR/${NAME}-macos-arm64"
echo "  -> $OUT_DIR/${NAME}-macos-arm64"

# macOS x86_64 (cross-compile)
echo ""
echo "--- macOS x64 (x86_64-apple-darwin) ---"
cargo build --release --target x86_64-apple-darwin
cp target/x86_64-apple-darwin/release/$NAME "$OUT_DIR/${NAME}-macos-x64"
echo "  -> $OUT_DIR/${NAME}-macos-x64"

# Linux ARM64 (cross-compile via zigbuild)
echo ""
echo "--- Linux arm64 (aarch64-unknown-linux-gnu) ---"
cargo zigbuild --release --target aarch64-unknown-linux-gnu
cp target/aarch64-unknown-linux-gnu/release/$NAME "$OUT_DIR/${NAME}-linux-arm64"
echo "  -> $OUT_DIR/${NAME}-linux-arm64"

# Linux x86_64 (cross-compile via zigbuild)
echo ""
echo "--- Linux x64 (x86_64-unknown-linux-gnu) ---"
cargo zigbuild --release --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/$NAME "$OUT_DIR/${NAME}-linux-x64"
echo "  -> $OUT_DIR/${NAME}-linux-x64"

echo ""
echo "=== Done ==="
ls -lh "$OUT_DIR"/${NAME}-*
