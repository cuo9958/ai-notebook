#!/bin/bash
set -e

echo "=========================================="
echo "Markdown Server Build Script"
echo "=========================================="

TARGET=${1:-default}

echo "[1/3] Formatting code..."
cargo fmt

echo "[2/3] Analyzing code..."
cargo clippy --no-deps || true

if [ "$TARGET" = "linux-musl" ]; then
    echo "[3/3] Building for Linux (musl static)..."
    cargo build --release --target x86_64-unknown-linux-musl
elif [ "$TARGET" = "windows-gnu" ]; then
    echo "[3/3] Building for Windows (cross)..."
    cargo build --release --target x86_64-pc-windows-gnu
else
    echo "[3/3] Building binary..."
    cargo build --release
fi

echo "Build complete!"
ls -lh target/*/release/markdown-server* || true
