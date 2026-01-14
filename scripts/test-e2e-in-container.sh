#!/bin/bash
set -e

# Host script to run E2E tests inside the docker container

IMAGE="omnectshareddevacr.azurecr.io/rust:bookworm"

echo "🐳 Launching test container..."

# Check if we need to build the frontend first
if [ ! -d "src/ui/dist" ]; then
    echo "📦 Building frontend..."
    ./scripts/build-frontend.sh
fi

# Run the test script inside the container
# We mount the current directory to /workspace
docker run --rm \
    -v $(pwd):/workspace \
    -w /workspace \
    --net=host \
    $IMAGE \
    /bin/bash -c "./scripts/run-e2e-tests.sh"
