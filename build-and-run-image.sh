#!/bin/bash
# file used for local development - builds and runs for current host platform

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Determine host architecture
HOST_ARCH="$(uname -m)"
case "$HOST_ARCH" in
  x86_64) ARCH="amd64" ;;
  aarch64|arm64) ARCH="arm64" ;;
  *) echo "Unsupported architecture: $HOST_ARCH"; exit 1 ;;
esac

# Configuration
IMAGE_TAG="${IMAGE_TAG:-local}"
UI_PORT="${UI_PORT:-1977}"
CENTRIFUGO_PORT="${CENTRIFUGO_PORT:-8000}"

# Build image using the main build script
echo "Building image for $ARCH architecture..."
"$SCRIPT_DIR/build-and-deploy-image.sh" --arch "$ARCH" --tag "$IMAGE_TAG"

IMAGE_NAME="omnectshareddevacr.azurecr.io/omnect-ui:${IMAGE_TAG}"

# Ensure required directories exist
mkdir -p "$SCRIPT_DIR/temp/data"
mkdir -p "$SCRIPT_DIR/temp/network"

# ensure presence of:
# /tmp/api.sock (normally created by a local instance of omnect-device-service)
# ./temp/cert.pem and ./temp/key.pem (certificate and key file)
echo "Running container..."
docker run --rm \
  -v "$SCRIPT_DIR/temp:/cert" \
  -v /tmp:/socket \
  -v "$SCRIPT_DIR/temp/data:/data" \
  -v "$SCRIPT_DIR/temp/network:/network" \
  -u "$(id -u):$(id -g)" \
  -e RUST_LOG=debug \
  -e UI_PORT="$UI_PORT" \
  -e SOCKET_PATH=/socket/api.sock \
  -e CENTRIFUGO_ADMIN_ENABLED=true \
  -e CENTRIFUGO_ADMIN_PASSWORD=123 \
  -e CENTRIFUGO_ADMIN_SECRET=123 \
  -e DATA_DIR_PATH=/data \
  -e KEYCLOAK_URL=https://keycloak.omnect.conplement.cloud/realms/cp-dev \
  -e TENANT=cp \
  -p "${UI_PORT}:${UI_PORT}" \
  -p "${CENTRIFUGO_PORT}:${CENTRIFUGO_PORT}" \
  "$IMAGE_NAME"
