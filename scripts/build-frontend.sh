#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Ensure we are in the project root
cd "$(dirname "$0")"/..

# Build WASM module
echo -n "Building WASM module... "
cd src/app
wasm-pack build --target web --out-dir ../ui/src/core/pkg >/dev/null 2>&1
cd ../..
echo -e "${GREEN}✓${NC}"

# Generate TypeScript types
echo -n "Generating TypeScript types... "
cargo build -p shared_types >/dev/null 2>&1
# Remove .js files to force Vite to use .ts sources
find src/shared_types/generated/typescript -name "*.js" -delete
echo -e "${GREEN}✓${NC}"

# Build UI
echo -n "Building UI... "
cd src/ui
bun install --frozen-lockfile >/dev/null 2>&1
bun run build >/dev/null 2>&1
cd ../..
echo -e "${GREEN}✓${NC}"

echo -e "${GREEN}✅ Frontend build complete!${NC}"