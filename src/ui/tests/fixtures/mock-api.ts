import { Page } from '@playwright/test';
import jwt from 'jsonwebtoken';

export async function mockConfig(page: Page) {
  const config = {
    KEYCLOAK_URL: 'http://localhost:8080',
    REALM: 'omnect',
    CLIENT_ID: 'omnect-ui',
    CENTRIFUGO_URL: 'wss://localhost:8000/connection/websocket'
  };

  // Add as init script so it's available even before config.js loads
  await page.addInitScript((cfg) => {
    (window as any).__APP_CONFIG__ = cfg;
  }, config);

  // Still mock the file request to avoid 404s
  await page.route('**/config.js', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/javascript',
      body: `window.__APP_CONFIG__ = ${JSON.stringify(config)};`,
    });
  });
}

export async function mockLoginSuccess(page: Page) {
  const token = jwt.sign({ sub: 'user123' }, 'secret', { expiresIn: '1h' });
  await page.route('**/token/login', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'text/plain',
      body: token,
    });
  });
}

export async function mockRequireSetPassword(page: Page) {
  await page.route('**/require-set-password', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: 'false',
    });
  });
}

export async function mockNetworkConfig(page: Page) {
  // Mock the network configuration endpoint
  // Note: The Core sends POST to /network, not api/v1/...
  await page.route('**/network', async (route) => {
    if (route.request().method() === 'POST') {
        // Mock successful application of network config
        await route.fulfill({
            status: 200,
            contentType: 'application/json',
            body: JSON.stringify({
                rollbackTimeoutSeconds: 90,
                uiPort: 5173,
                rollbackEnabled: true
            }),
        });
    } else {
        // Fallback for other methods if any
        await route.continue();
    }
  });
}
