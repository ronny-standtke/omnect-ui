#!/bin/bash
set -e

# Host script to run E2E tests inside the docker container

# Navigate to repository root (parent of scripts directory)
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

IMAGE="omnectweucopsacr.azurecr.io/rust:bookworm"

echo "🐳 Launching test container..."

# Check if we need to build the frontend first
if [ ! -d "src/ui/dist" ]; then
    echo "📦 Building frontend..."
    ./scripts/build-frontend.sh
fi

# Run the test script inside the container
# We mount the repository root to /workspace
docker run --rm \
    -v "$REPO_ROOT":/workspace \
    -w /workspace \
    --net=host \
    $IMAGE \
    ./scripts/run-e2e-tests.sh "$@"
