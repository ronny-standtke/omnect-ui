#!/bin/bash
# Development setup script for omnect-ui
# This script performs the same setup as the VSCode pre-launch task

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

echo "🔧 Running development setup..."

# Check if omnect-device-service is running
echo -n "Checking omnect-device-service... "
if [ ! -S /tmp/api.sock ]; then
    echo -e "${RED}❌ ERROR${NC}"
    echo "omnect-device-service is not running!"
    echo "Please start it first from your omnect-device-service directory"
    echo "See: https://github.com/omnect/omnect-device-service"
    exit 1
else
    echo -e "${GREEN}✓${NC}"
fi

# Kill any existing centrifugo processes
echo -n "Stopping existing centrifugo processes... "
killall centrifugo 2>/dev/null || true
echo -e "${GREEN}✓${NC}"

# Setup test password
echo -n "Setting up test password... "
cargo run --bin setup-password --features=mock -- 123 >/dev/null 2>&1
echo -e "${GREEN}✓${NC}"

echo -e "${GREEN}✅ Development setup complete!${NC}"
echo ""
echo "You can now run the application with:"
echo "  cargo run --bin omnect-ui --features=mock"
