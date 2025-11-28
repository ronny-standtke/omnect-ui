#!/bin/bash
# Generate TypeScript types from Rust and convert to ESM
#
# TypeGen outputs CommonJS by default, but Vite requires ESM.
# This script regenerates the types and recompiles them as ESM modules.

set -e

echo "Generating TypeScript types from Rust..."
cargo build -p shared_types

echo "Converting TypeScript compilation to ESM..."
cd src/shared_types/generated/typescript

# Update tsconfig to output ES modules instead of CommonJS
sed -i 's/"module": "commonjs"/"module": "esnext"/' tsconfig.json

# Recompile TypeScript to JavaScript with ES module syntax
export PATH="$HOME/.local/share/pnpm:$PATH"
pnpm exec tsc

echo "TypeScript types generated and converted to ESM successfully!"
