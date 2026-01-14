#!/bin/bash
set -e

# Internal script to run E2E tests inside the container

echo "🔧 Setting up test environment..."

# 0. Ensure bun is installed (needed for UI)
if ! command -v bun &> /dev/null; then
    echo "⚠️ Bun not found, installing..."
    curl -fsSL https://bun.sh/install | bash
    export BUN_INSTALL="$HOME/.bun"
    export PATH="$BUN_INSTALL/bin:$PATH"
fi

# 1. Ensure Centrifugo is available (using the tool script if needed)
if ! command -v centrifugo &> /dev/null; then
    echo "⚠️ Centrifugo not found in PATH, checking tools directory..."
    if [ ! -f "tools/centrifugo" ]; then
        ./tools/setup-centrifugo.sh
    fi
    export PATH=$PATH:$(pwd)/tools
fi

# 2. Start Centrifugo directly (Backend is mocked, but we need real WS)
echo "🚀 Starting Centrifugo..."
# Using the config from backend/config/centrifugo_config.json
CENTRIFUGO_CONFIG="src/backend/config/centrifugo_config.json"

# Generate self-signed certs for testing if missing
mkdir -p temp/certs
if [ ! -f "temp/certs/server.cert.pem" ] || [ ! -r "temp/certs/server.key.pem" ]; then
    echo "🔐 Generating self-signed certificates..."
    # Check if old certs exist with wrong permissions
    if [ -f "temp/certs/server.cert.pem" ] && [ ! -w "temp/certs/server.cert.pem" ]; then
        echo "❌ Error: Old certificates exist with wrong permissions (likely created by root)"
        echo "   Please run: sudo rm -rf temp/certs"
        exit 1
    fi
    rm -f temp/certs/server.cert.pem temp/certs/server.key.pem
    openssl req -newkey rsa:2048 -nodes -keyout temp/certs/server.key.pem -x509 -days 365 -out temp/certs/server.cert.pem -subj "/CN=localhost" 2>/dev/null
    chmod 644 temp/certs/server.key.pem temp/certs/server.cert.pem
fi

# Env vars for Centrifugo
export CENTRIFUGO_HTTP_SERVER_TLS_CERT_PEM="temp/certs/server.cert.pem"
export CENTRIFUGO_HTTP_SERVER_TLS_KEY_PEM="temp/certs/server.key.pem"
export CENTRIFUGO_HTTP_SERVER_PORT="8000"
export CENTRIFUGO_CLIENT_TOKEN_HMAC_SECRET_KEY="secret"
export CENTRIFUGO_HTTP_API_KEY="api_key"
export CENTRIFUGO_LOG_LEVEL="info"

centrifugo -c "$CENTRIFUGO_CONFIG" > /tmp/centrifugo.log 2>&1 &
CENTRIFUGO_PID=$!

echo "⏳ Waiting for Centrifugo..."
for i in {1..30}; do
    if curl -k -s https://localhost:8000/health > /dev/null; then
        echo "✅ Centrifugo is ready!"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "❌ Centrifugo failed to start."
        cat /tmp/centrifugo.log
        kill $CENTRIFUGO_PID || true
        exit 1
    fi
    sleep 1
done

# 3. Serve the Frontend
echo "🌐 Starting Frontend Dev Server..."
cd src/ui

# Check for permission issues with Vite cache
if [ -d "node_modules/.vite" ] && [ ! -w "node_modules/.vite" ]; then
    echo "❌ Error: Vite cache has wrong permissions (likely created by root)"
    echo "   Please run: sudo rm -rf src/ui/node_modules/.vite"
    kill $CENTRIFUGO_PID || true
    exit 1
fi

# Install dependencies if needed (container might not have node_modules)
if [ ! -d "node_modules" ]; then
    echo "📦 Installing UI dependencies..."
    bun install
fi

# Start vite dev server in background
bun run dev --port 5173 > /tmp/vite.log 2>&1 &
FRONTEND_PID=$!

# Wait for Frontend
echo "⏳ Waiting for Frontend..."
for i in {1..30}; do
    if curl -s http://localhost:5173 > /dev/null; then
        echo "✅ Frontend is ready!"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "❌ Frontend failed to start."
        cat /tmp/vite.log
        kill $FRONTEND_PID || true
        kill $CENTRIFUGO_PID || true
        exit 1
    fi
    sleep 1
done

# 4. Run Playwright Tests
echo "🧪 Running Playwright Tests..."

# Check for permission issues with Playwright test results
if [ -d "test-results" ] && [ ! -w "test-results" ]; then
    echo "❌ Error: Playwright test-results directory has wrong permissions (likely created by root)"
    echo "   Please run: sudo rm -rf src/ui/test-results src/ui/playwright-report"
    kill $FRONTEND_PID || true
    kill $CENTRIFUGO_PID || true
    exit 1
fi

# Install Playwright browsers (always run to ensure correct version)
echo "📦 Ensuring Playwright browsers are installed..."
npx playwright install chromium

# BASE_URL is set for playwright.config.ts
export BASE_URL="http://localhost:5173"

# Run tests
npx playwright test

TEST_EXIT_CODE=$?

exit $TEST_EXIT_CODE
