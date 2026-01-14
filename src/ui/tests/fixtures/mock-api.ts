import { Page } from '@playwright/test';

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
  await page.route('**/token/login', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'text/plain',
      body: 'mock_token_123',
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
  await page.route('**/api/v1/network/config', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        interfaces: [
          {
            name: 'eth0',
            dhcp: true,
          },
        ],
      }),
    });
  });
}
