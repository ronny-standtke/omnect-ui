import { test, expect } from '@playwright/test';
import { mockConfig, mockLoginSuccess, mockRequireSetPassword } from './fixtures/mock-api';
import { NetworkTestHarness } from './fixtures/network-test-harness';

test.describe('Network Rollback Status', () => {
  let harness: NetworkTestHarness;

  test.beforeEach(async () => {
    harness = new NetworkTestHarness();
  });

  test.afterEach(() => {
    harness.reset();
  });

  test('rollback status is cleared after ack and does not reappear on re-login', async ({ page }) => {
    let healthcheckRollbackStatus = true;

    await mockConfig(page);
    await mockLoginSuccess(page);
    await mockRequireSetPassword(page);

    await page.route('**/healthcheck', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          version_info: { required: '>=0.39.0', current: '0.40.0', mismatch: false },
          update_validation_status: { status: 'valid' },
          network_rollback_occurred: healthcheckRollbackStatus,
        }),
      });
    });

    await page.route('**/ack-rollback', async (route) => {
      if (route.request().method() === 'POST') {
        healthcheckRollbackStatus = false;
        await route.fulfill({ status: 200 });
      }
    });

    await page.goto('/');
    await expect(page.getByText('Network Settings Rolled Back')).toBeVisible({ timeout: 10000 });

    await page.getByRole('button', { name: /ok/i }).click();
    await expect(page.getByText('Network Settings Rolled Back')).not.toBeVisible();
    await page.waitForTimeout(500);

    await page.getByPlaceholder(/enter your password/i).fill('password');
    await page.getByRole('button', { name: /log in/i }).click();
    await expect(page.getByText('Common Info')).toBeVisible({ timeout: 10000 });

    await harness.setup(page, {
      ipv4: {
        addrs: [{ addr: 'localhost', dhcp: false, prefix_len: 24 }],
        dns: ['8.8.8.8'],
        gateways: ['192.168.1.1'],
      },
    });

    await page.reload();
    await expect(page.getByText('Network Settings Rolled Back')).not.toBeVisible({ timeout: 3000 });

    await page.getByPlaceholder(/enter your password/i).fill('password');
    await page.getByRole('button', { name: /log in/i }).click();
    await expect(page.getByText('Common Info')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Network Settings Rolled Back')).not.toBeVisible();

    await harness.navigateToNetwork(page);
    await harness.navigateToAdapter(page, 'eth0');
    await expect(page.getByText('eth0')).toBeVisible();
  });
});

test.describe('Network Rollback Defaults', () => {
  let harness: NetworkTestHarness;

  test.beforeEach(async ({ page }) => {
    harness = new NetworkTestHarness();
    await mockConfig(page);
    await mockLoginSuccess(page);
    await mockRequireSetPassword(page);
    await harness.mockNetworkConfig(page);
    await harness.mockHealthcheck(page);

    await page.goto('/');
    await page.getByPlaceholder(/enter your password/i).fill('password');
    await page.getByRole('button', { name: /log in/i }).click();
    await expect(page.getByText('Common Info')).toBeVisible();
  });

  test.afterEach(() => {
    harness.reset();
  });

  test('Static -> DHCP: Rollback should be DISABLED by default', async ({ page }) => {
    await harness.setup(page, { ipv4: { addrs: [{ addr: 'localhost', dhcp: false, prefix_len: 24 }] } });
    await expect(page.getByLabel('Static')).toBeChecked();

    await page.getByLabel('DHCP').click({ force: true });
    await page.getByRole('button', { name: /save/i }).click();

    await expect(page.getByText('Confirm Network Configuration Change')).toBeVisible();
    await expect(page.getByRole('checkbox', { name: /Enable automatic rollback/i })).not.toBeChecked();
  });

  test('DHCP -> Static: Rollback should be ENABLED by default', async ({ page }) => {
    await harness.setup(page, { ipv4: { addrs: [{ addr: 'localhost', dhcp: true, prefix_len: 24 }] } });
    await expect(page.getByLabel('DHCP')).toBeChecked();

    await page.getByLabel('Static').click({ force: true });
    await page.getByRole('textbox', { name: /IP Address/i }).fill('192.168.1.150');
    await page.getByRole('button', { name: /save/i }).click();

    await expect(page.getByText('Confirm Network Configuration Change')).toBeVisible();
    await expect(page.getByRole('checkbox', { name: /Enable automatic rollback/i })).toBeChecked();
  });

  test('DHCP -> Static (Same IP): Rollback should be ENABLED', async ({ page }) => {
    await harness.setup(page, { ipv4: { addrs: [{ addr: 'localhost', dhcp: true, prefix_len: 24 }] } });
    await expect(page.getByLabel('DHCP')).toBeChecked();

    await page.getByLabel('Static').click({ force: true });
    // IP is auto-filled with current IP ('localhost'), do NOT change it.
    await page.getByRole('button', { name: /save/i }).click();

    // Verify Modal
    await expect(page.getByText('Confirm Network Configuration Change')).toBeVisible();

    // Verify Checkbox is CHECKED
    await expect(page.getByRole('checkbox', { name: /Enable automatic rollback/i })).toBeChecked();

    // Apply changes
    await page.getByRole('button', { name: /apply changes/i }).click();

    // Verify overlay appears with countdown label
    await expect(page.locator('#overlay').getByText('Automatic rollback in:')).toBeVisible({ timeout: 10000 });
  });

  test('Rollback should show MODAL not SNACKBAR when connection is restored at old IP', async ({ page }) => {
    await harness.setup(page, { ipv4: { addrs: [{ addr: 'localhost', dhcp: false, prefix_len: 24 }] } });

    await page.getByLabel('DHCP').click({ force: true });
    
    // Mock /network to return a short rollback timeout for testing
    await harness.mockNetworkConfig(page, { rollbackTimeoutSeconds: 2 });

    await page.getByRole('button', { name: /save/i }).click();
    await expect(page.getByText('Confirm Network Configuration Change')).toBeVisible();
    await page.getByRole('checkbox', { name: /Enable automatic rollback/i }).check();
    
    // Override healthcheck mock to return network_rollback_occurred: true
    await page.unroute('**/healthcheck');
    await page.route('**/healthcheck', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          version_info: { required: '>=0.39.0', current: '0.40.0', mismatch: false },
          update_validation_status: { status: 'valid' },
          network_rollback_occurred: true,
        }),
      });
    });

    await page.getByRole('button', { name: /Apply Changes/i }).click();
    await expect(page.getByText('Applying network settings')).toBeVisible();
    
    await expect(page.getByText('Automatic network rollback successful')).not.toBeVisible();
    await expect(page.getByText('The network settings were rolled back to the previous configuration')).toBeVisible({ timeout: 10000 });
  });
});

test.describe('Network Rollback Regression', () => {
  let harness: NetworkTestHarness;

  test.beforeEach(async ({ page }) => {
    harness = new NetworkTestHarness();

    // Shim WebSocket to redirect 192.168.1.150 to localhost
    await page.addInitScript(() => {
        const OriginalWebSocket = window.WebSocket;
        // @ts-ignore
        window.WebSocket = function(url, protocols) {
            if (typeof url === 'string' && url.includes('192.168.1.150')) {
                console.log(`[Shim] Redirecting WebSocket ${url} to localhost`);
                url = url.replace('192.168.1.150', 'localhost');
            }
            return new OriginalWebSocket(url, protocols);
        };
        window.WebSocket.prototype = OriginalWebSocket.prototype;
        window.WebSocket.CONNECTING = OriginalWebSocket.CONNECTING;
        window.WebSocket.OPEN = OriginalWebSocket.OPEN;
        window.WebSocket.CLOSING = OriginalWebSocket.CLOSING;
        window.WebSocket.CLOSED = OriginalWebSocket.CLOSED;
    });

    // Mock the redirect to the new IP to prevent navigation failure
    await page.route(/.*192\.168\.1\.150.*/, async (route) => {
        const url = new URL(route.request().url());
        
        // Handle healthcheck separately to avoid CORS and ensure correct response
        if (url.pathname.includes('/healthcheck')) {
            await route.fulfill({
                status: 200,
                contentType: 'application/json',
                headers: {
                    'Access-Control-Allow-Origin': 'https://localhost:5173', // or '*'
                    'Access-Control-Allow-Credentials': 'true',
                },
                body: JSON.stringify({
                    version_info: { required: '>=0.39.0', current: '0.40.0', mismatch: false },
                    update_validation_status: { status: 'valid' },
                    network_rollback_occurred: harness.getRollbackState().occurred,
                }),
            });
            return;
        }

        // For other requests (main page, assets), proxy to localhost
        const originalUrl = page.url(); 
        const originalPort = new URL(originalUrl).port || '5173';
        const originalProtocol = new URL(originalUrl).protocol;
        
        const newUrl = `${originalProtocol}//localhost:${originalPort}${url.pathname}${url.search}`;
        
        console.log(`Redirecting ${url.href} to ${newUrl}`);
        
        try {
            const response = await page.request.fetch(newUrl, {
                method: route.request().method(),
                headers: route.request().headers(),
                data: route.request().postDataBuffer(),
                ignoreHTTPSErrors: true
            });
            
            await route.fulfill({
                response: response
            });
        } catch (e) {
            console.error('Failed to proxy request:', e);
            await route.abort();
        }
    });

    await mockConfig(page);
    await mockLoginSuccess(page);
    await mockRequireSetPassword(page);
    await harness.mockNetworkConfig(page, { rollbackTimeoutSeconds: 10 }); // Short timeout
    await harness.mockHealthcheck(page);
    await harness.mockAckRollback(page);

    await page.goto('/');
    await page.getByPlaceholder(/enter your password/i).fill('password');
    await page.getByRole('button', { name: /log in/i }).click();
    await expect(page.getByText('Common Info')).toBeVisible({ timeout: 10000 });
  });

  test.afterEach(() => {
    harness.reset();
  });

  test('Rollback modal should close on second apply after a rollback', async ({ page }) => {
    test.setTimeout(60000); // Increase timeout for rollback scenario

    // Set browser hostname to match the adapter IP so Core detects it as current connection
    await page.evaluate(() => {
        // @ts-ignore
        if (window.setBrowserHostname) {
            // @ts-ignore
            window.setBrowserHostname('localhost');
        }
    });

    // 1. Setup adapter with static IP (current connection)
    await harness.setup(page, {
      ipv4: {
        addrs: [{ addr: 'localhost', dhcp: false, prefix_len: 24 }],
        dns: ['8.8.8.8'],
        gateways: ['192.168.1.1'],
      },
    });

    // 2. First Change - will trigger rollback
    const ipInput = page.getByRole('textbox', { name: /IP Address/i }).first();
    await ipInput.fill('192.168.1.150');

    await page.getByRole('button', { name: /save/i }).click();

    const confirmDialog = page.getByText('Confirm Network Configuration Change');
    await expect(confirmDialog).toBeVisible();

    const rollbackCheckbox = page.getByRole('checkbox', { name: /Enable automatic rollback/i });
    if (!(await rollbackCheckbox.isChecked())) {
        await rollbackCheckbox.check();
    }

    await page.getByRole('button', { name: /apply changes/i }).click();

    // Verify modal closed
    await expect(confirmDialog).not.toBeVisible();

    // Verify rollback overlay appears
    await expect(page.locator('#overlay').getByText('Automatic rollback in:')).toBeVisible();

    // Wait for at least one healthcheck attempt on the new IP
    await page.waitForResponse(resp => resp.url().includes('192.168.1.150') && resp.url().includes('healthcheck'));

    // 3. Simulate Rollback Timeout
    await harness.simulateRollbackTimeout();

    // UI should eventually show "Network Settings Rolled Back"
    await expect(page.getByText('Network Settings Rolled Back')).toBeVisible({ timeout: 30000 });

    // 4. Acknowledge Rollback
    await page.getByRole('button', { name: /ok/i }).click();

    // Verify we are back to normal
    await expect(page.getByText('Network Settings Rolled Back')).not.toBeVisible();

    // Handle re-login if necessary
    if (await page.getByRole('button', { name: /log in/i }).isVisible()) {
        await page.getByPlaceholder(/enter your password/i).fill('password');
        await page.getByRole('button', { name: /log in/i }).click();
        await expect(page.getByText('Common Info')).toBeVisible({ timeout: 10000 });
    }

    // Force the browser hostname to match the harness IP.
    await page.evaluate((ip) => {
        // @ts-ignore
        if (window.setBrowserHostname) {
            // @ts-ignore
            window.setBrowserHostname(ip);
        }
    }, '192.168.1.100');

    // Ensure we are back on the adapter page
    if (!await page.getByRole('textbox', { name: /IP Address/i }).first().isVisible()) {
         await harness.navigateToNetwork(page);
         await harness.navigateToAdapter(page, 'eth0');
    }

    const currentIpInput = page.getByRole('textbox', { name: /IP Address/i }).first();
    await expect(currentIpInput).toHaveValue('192.168.1.100');

    // 5. Second Change - try again
    await currentIpInput.fill('192.168.1.151');
    await page.getByRole('button', { name: /save/i }).click();

    await expect(confirmDialog).toBeVisible();

    // Ensure rollback is checked
     if (!(await rollbackCheckbox.isChecked())) {
        await rollbackCheckbox.check();
    }

    await page.getByRole('button', { name: /apply changes/i }).click();

    // 6. Verify modal closed (second time)
    await expect(confirmDialog).not.toBeVisible({ timeout: 5000 });
  });
});

