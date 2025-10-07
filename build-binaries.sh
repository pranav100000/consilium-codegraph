#!/bin/bash
# Build binaries for all supported platforms and copy to ts-client/bin

set -e

echo "🔨 Building Consilium CodeGraph binaries for all platforms..."

# Current platform (always build this one)
echo ""
echo "📦 Building for current platform..."
cargo build --release

# Detect current platform
ARCH=$(uname -m)
OS=$(uname -s)

if [[ "$OS" == "Darwin" ]]; then
  if [[ "$ARCH" == "arm64" ]]; then
    PLATFORM="darwin-arm64"
  else
    PLATFORM="darwin-x64"
  fi
elif [[ "$OS" == "Linux" ]]; then
  PLATFORM="linux-x64"
else
  echo "⚠️  Unsupported platform: $OS $ARCH"
  exit 1
fi

echo "✅ Built for $PLATFORM"

# Copy to ts-client/bin
echo ""
echo "📋 Copying binary to ts-client/bin/$PLATFORM..."
mkdir -p ts-client/bin/$PLATFORM
cp target/release/reviewbot ts-client/bin/$PLATFORM/
chmod +x ts-client/bin/$PLATFORM/reviewbot

echo "✅ Binary copied to ts-client/bin/$PLATFORM/reviewbot"

# Note about cross-compilation
echo ""
echo "ℹ️  To build for other platforms:"
echo ""
echo "  macOS ARM64:  rustup target add aarch64-apple-darwin && cargo build --release --target aarch64-apple-darwin"
echo "  macOS Intel:  rustup target add x86_64-apple-darwin && cargo build --release --target x86_64-apple-darwin"
echo "  Linux:        rustup target add x86_64-unknown-linux-gnu && cargo build --release --target x86_64-unknown-linux-gnu"
echo "  Windows:      rustup target add x86_64-pc-windows-msvc && cargo build --release --target x86_64-pc-windows-msvc"
echo ""
echo "  Then copy to: ts-client/bin/<platform>/"
echo ""
echo "🎉 Done! Binary ready for distribution."
