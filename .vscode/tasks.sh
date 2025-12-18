#!/bin/bash

set -e

# Color codes
RED="\033[0;31m"
GREEN="\033[0;32m"
NC="\033[0m"

# Check omnect-device-service
echo -n "Checking omnect-device-service... "
if [ ! -S /tmp/api.sock ]; then
    echo -e "${RED}❌ ERROR${NC}"
    echo "omnect-device-service is not running!"
    echo "Please start it first from your omnect-device-service directory"
    echo "See: https://github.com/omnect/omnect-device-service"
    exit 1
fi

if ! pgrep -f "omnect-device-service" > /dev/null; then
    echo -e "${RED}❌ ERROR${NC}"
    echo "omnect-device-service process is not running!"
    echo "Please start it first from your omnect-device-service directory"
    echo "See: https://github.com/omnect/omnect-device-service"
    exit 1
fi

echo -e "${GREEN}✓${NC}"

# Stop existing centrifugo processes
echo -n "Stopping existing centrifugo processes... "
killall centrifugo 2>/dev/null || true
echo -e "${GREEN}✓${NC}"
