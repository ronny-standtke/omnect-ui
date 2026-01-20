#!/bin/bash
# Download Centrifugo binary for local development

set -e

# Navigate to repository root (parent of scripts directory)
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

CENTRIFUGO_VERSION="${CENTRIFUGO_VERSION:-v6.1.0}"
SCRIPT_DIR="tools"
CENTRIFUGO_BIN="$SCRIPT_DIR/centrifugo"

# Create directory if it doesn't exist
mkdir -p "$SCRIPT_DIR"

if [[ -f "$CENTRIFUGO_BIN" ]]; then
  echo "Centrifugo already exists at $CENTRIFUGO_BIN"
  "$CENTRIFUGO_BIN" version 2>/dev/null || true
  exit 0
fi

echo "Downloading Centrifugo $CENTRIFUGO_VERSION..."

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case $ARCH in
  x86_64)
    ARCH="amd64"
    ;;
  aarch64|arm64)
    ARCH="arm64"
    ;;
  *)
    echo "Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

# Download and extract
DOWNLOAD_URL="https://github.com/centrifugal/centrifugo/releases/download/${CENTRIFUGO_VERSION}/centrifugo_${CENTRIFUGO_VERSION#v}_${OS}_${ARCH}.tar.gz"

echo "Downloading from: $DOWNLOAD_URL"
curl -sSL "$DOWNLOAD_URL" | tar -xz -C "$SCRIPT_DIR" centrifugo

chmod +x "$CENTRIFUGO_BIN"
echo "Centrifugo installed successfully at $CENTRIFUGO_BIN"
"$CENTRIFUGO_BIN" version
