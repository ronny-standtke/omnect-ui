import { test, expect, Page } from '@playwright/test';
import { mockConfig, mockLoginSuccess, mockRequireSetPassword } from './fixtures/mock-api';
import { NetworkTestHarness, DeviceNetwork } from './fixtures/network-test-harness';

// Run all tests in this file serially to avoid Centrifugo channel interference
test.describe.configure({ mode: 'serial' });

test.describe('Network Configuration - Comprehensive E2E Tests', () => {
  let harness: NetworkTestHarness;

  test.beforeEach(async ({ page }) => {
    harness = new NetworkTestHarness();
    await mockConfig(page);
    await mockLoginSuccess(page);
    await mockRequireSetPassword(page);
    await harness.mockNetworkConfig(page);
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

  test.describe('CRITICAL: Rollback Flows and Error Handling', () => {
    test('automatic rollback timeout - healthcheck fails, rollback triggered', async ({ page }) => {
      const shortTimeoutSeconds = 3;
      await page.unroute('**/network');
      await harness.mockNetworkConfig(page, { rollbackTimeoutSeconds: shortTimeoutSeconds });
      await harness.mockHealthcheck(page, { healthcheckAlwaysFails: true });

      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: 'localhost', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      await expect(page.getByText('(current connection)')).toBeVisible();

      // Change IP address
      const ipInput = page.getByRole('textbox', { name: /IP Address/i }).first();
      await ipInput.fill('192.168.1.150');

      await page.getByRole('button', { name: /save/i }).click();

      await expect(page.getByText('Confirm Network Configuration Change')).toBeVisible({ timeout: 5000 });
      await expect(page.getByRole('checkbox', { name: /Enable automatic rollback/i })).toBeChecked();

      await page.getByRole('button', { name: /apply changes/i }).click();

      await expect(page.locator('#overlay').getByText('Automatic rollback in:')).toBeVisible({ timeout: 10000 });

      expect(harness.getRollbackState().enabled).toBe(true);

      await harness.simulateRollbackTimeout();
      // Wait long enough for client-side timeout (3s) to fire, which triggers the UI update
      // Increased to 6s for stability in slow environments
      await page.waitForTimeout(6000);

      await harness.mockHealthcheck(page, { healthcheckAlwaysFails: false });
      // Explicitly fail the new IP healthcheck to force timeout/rollback logic on client
      // We use abort() to simulate a network error because the app masks 503 responses as 200
      await page.route('**/*192.168.1.150*/healthcheck', route => route.abort());

      await expect(page.locator('#overlay').getByText(/Automatic rollback initiated/i).first()).toBeVisible({ timeout: 15000 });
      await expect(page.locator('#overlay')).not.toBeVisible({ timeout: 20000 });
      await expect(page).toHaveURL(/\/login/, { timeout: 15000 });
    });

    test('DHCP rollback - automatic redirect to login after timeout', async ({ page }) => {
      const shortTimeoutSeconds = 3;
      await page.unroute('**/network');
      await harness.mockNetworkConfig(page, { rollbackTimeoutSeconds: shortTimeoutSeconds });
      // Ensure healthcheck fails initially so we can verify the timeout/rollback state
      await harness.mockHealthcheck(page, { healthcheckAlwaysFails: true });

      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: 'localhost', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      await page.getByLabel('DHCP').click({ force: true });
      await page.waitForTimeout(500);
      await page.getByRole('button', { name: /save/i }).click();

      await expect(page.getByText('Confirm Network Configuration Change')).toBeVisible({ timeout: 5000 });
      await page.getByRole('checkbox', { name: /Enable automatic rollback/i }).check();
      await page.getByRole('button', { name: /apply changes/i }).click();

      await expect(page.locator('#overlay')).toBeVisible();

      await harness.simulateRollbackTimeout();
      // Wait for client timeout (3s) to trigger UI update
      await page.waitForTimeout(4000);

      // Verify rollback message matches
      await expect(page.locator('#overlay').getByText(/Automatic rollback initiated/i).first()).toBeVisible({ timeout: 15000 });

      // Now allow recovery
      await harness.mockHealthcheck(page, { healthcheckAlwaysFails: false });

      await expect(page.locator('#overlay')).not.toBeVisible({ timeout: 20000 });
      await expect(page).toHaveURL(/\/login/, { timeout: 10000 });
      // Changed: expect MODAL instead of snackbar (fix: show rollback modal instead of snackbar on dynamic rollback)
      await expect(page.getByText('Network Settings Rolled Back')).toBeVisible();
    });

    test('rollback cancellation - new IP becomes reachable within timeout', async ({ page }) => {
      await harness.mockHealthcheck(page, { healthcheckSuccessAfter: 2000 });

      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: 'localhost', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      const ipInput = page.getByRole('textbox', { name: /IP Address/i }).first();
      await ipInput.fill('192.168.1.150');

      await page.getByRole('button', { name: /save/i }).click();
      await expect(page.getByText('Confirm Network Configuration Change')).toBeVisible({ timeout: 5000 });
      await page.getByRole('button', { name: /apply changes/i }).click();

      await expect(page.locator('#overlay').getByText('Automatic rollback in:')).toBeVisible({ timeout: 10000 });

      await page.waitForTimeout(3000);
      await harness.simulateNewIpReachable();
      await page.waitForTimeout(1000);

      const rollbackState = harness.getRollbackState();
      expect(rollbackState.enabled).toBe(false);
      expect(rollbackState.occurred).toBe(false);
    });

    test('invalid IP address validation error', async ({ page }) => {
      await harness.setup(page, {}); // default adapter

      const ipInput = page.getByRole('textbox', { name: /IP Address/i }).first();
      await page.getByLabel('Static').click({ force: true });

      await ipInput.fill('999.999.999.999');
      await expect(ipInput).toBeVisible(); // Vuetify error state check usually involves class or error message

      await ipInput.fill('not.an.ip.address');
      await expect(ipInput).toHaveValue('not.an.ip.address');

      await ipInput.fill('192.168.1.200');
      await expect(ipInput).toHaveValue('192.168.1.200');
    });

    test('backend error handling during configuration apply', async ({ page }) => {
      await harness.mockNetworkConfigError(page, 500, 'Failed to apply network configuration. Please check your settings and try again.');

      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: '192.168.1.200', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      const ipInput = page.getByRole('textbox', { name: /IP Address/i }).first();
      await ipInput.fill('192.168.1.210');

      const saveButton = page.getByRole('button', { name: /save/i });
      await saveButton.click();

      await expect(saveButton).toBeEnabled({ timeout: 5000 });
    });

    test('REGRESSION: form fields not reset during editing (caret stability)', async ({ page }) => {
      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: '192.168.1.100', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      const ipInput = page.getByRole('textbox', { name: /IP Address/i }).first();
      await expect(ipInput).toHaveValue('192.168.1.100', { timeout: 5000 });

      await ipInput.clear();
      await ipInput.pressSequentially('10.20.30.40', { delay: 50 });
      await expect(ipInput).toHaveValue('10.20.30.40');

      await page.getByLabel('DHCP').click({ force: true });
      await expect(page.getByLabel('DHCP')).toBeChecked();

      await page.getByLabel('Static').click({ force: true });
      await expect(page.getByLabel('Static')).toBeChecked();
      await expect(ipInput).toBeEditable();

      await ipInput.clear();
      await ipInput.pressSequentially('172.16.0.1', { delay: 50 });
      await expect(ipInput).toHaveValue('172.16.0.1');
    });

    test('REGRESSION: Save on non-current adapter should not show endless progress', async ({ page }) => {
      // Setup two adapters: eth0 (current) and eth1 (not current)
      await harness.setup(page, [
        { name: 'eth0', ipv4: { addrs: [{ addr: 'localhost', dhcp: false, prefix_len: 24 }] } },
        { name: 'eth1', mac: '00:11:22:33:44:56', ipv4: { addrs: [{ addr: '192.168.1.101', dhcp: false, prefix_len: 24 }] } }
      ], 'eth1');

      // Verify it's not the current connection
      await expect(page.getByText('(current connection)')).not.toBeVisible();

      const ipInput = page.getByRole('textbox', { name: /IP Address/i }).first();
      await ipInput.fill('192.168.1.102'); // Change static IP

      // Use the new helper to click Save and verify it finishes
      await harness.saveAndVerify(page);

      // Verify IP field is still editable
      await expect(ipInput).toBeEditable();

      // Switch to DHCP
      await page.locator('.v-window-item--active').getByLabel('DHCP').click({ force: true });
      await expect(page.locator('.v-window-item--active').getByLabel('DHCP')).toBeChecked();

      // Verify IP field is NOT editable (or hidden/disabled)
      await expect(ipInput).not.toBeEditable();

      await harness.saveAndVerify(page);
    });
  });

  test.describe('HIGH: Basic Configuration Workflows', () => {
    test('static IP on non-server adapter - no rollback modal', async ({ page }) => {
      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: '192.168.1.200', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      const ipInput = page.getByRole('textbox', { name: /IP Address/i }).first();
      await ipInput.fill('192.168.1.210');

      await page.getByRole('button', { name: /save/i }).click();
      await page.waitForTimeout(500);
      await expect(page.getByText('Confirm Network Configuration Change')).not.toBeVisible();
    });

    test('static IP on server adapter with rollback enabled', async ({ page }) => {
      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: 'localhost', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      await expect(page.getByText('(current connection)')).toBeVisible();

      const ipInput = page.getByRole('textbox', { name: /IP Address/i }).first();
      await ipInput.fill('192.168.1.150');

      await page.getByRole('button', { name: /save/i }).click();

      await expect(page.getByText('Confirm Network Configuration Change')).toBeVisible({ timeout: 5000 });
      await expect(page.getByRole('checkbox', { name: /Enable automatic rollback/i })).toBeChecked();

      await page.getByRole('button', { name: /apply changes/i }).click();
      await expect(page.locator('#overlay').getByText('Automatic rollback in:')).toBeVisible({ timeout: 10000 });
      expect(harness.getRollbackState().enabled).toBe(true);
    });

    test('static IP on server adapter with rollback disabled', async ({ page }) => {
      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: 'localhost', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      const ipInput = page.getByRole('textbox', { name: /IP Address/i }).first();
      await ipInput.fill('192.168.1.150');

      await page.getByRole('button', { name: /save/i }).click();

      await expect(page.getByText('Confirm Network Configuration Change')).toBeVisible({ timeout: 5000 });
      await page.getByRole('checkbox', { name: /Enable automatic rollback/i }).uncheck();

      await page.getByRole('button', { name: /apply changes/i }).click();
      await page.waitForTimeout(500);
      expect(harness.getRollbackState().enabled).toBe(false);
    });

    test('DHCP on non-server adapter', async ({ page }) => {
      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: '192.168.1.200', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      await page.getByLabel('DHCP').click({ force: true });
      await page.waitForTimeout(300);
      await page.getByRole('button', { name: /save/i }).click();

      await page.waitForTimeout(500);
      await expect(page.getByText('Confirm Network Configuration Change')).not.toBeVisible();
    });

    test('DHCP on server adapter with rollback enabled', async ({ page }) => {
      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: 'localhost', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      await expect(page.getByRole('textbox', { name: /IP Address/i }).first()).toHaveValue('localhost', { timeout: 8000 });

      await page.getByLabel('DHCP').click({ force: true });
      await page.waitForTimeout(300);
      await page.getByRole('button', { name: /save/i }).click();

      await expect(page.getByText('Confirm Network Configuration Change')).toBeVisible({ timeout: 5000 });
      await expect(page.getByRole('checkbox', { name: /Enable automatic rollback/i })).not.toBeChecked();

      await page.getByRole('checkbox', { name: /Enable automatic rollback/i }).check();
      await page.getByRole('button', { name: /apply changes/i }).click();
      await page.waitForTimeout(1000);
      expect(harness.getRollbackState().enabled).toBe(true);
    });

    test('DHCP on server adapter with rollback disabled', async ({ page }) => {
      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: 'localhost', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      await page.getByLabel('DHCP').click({ force: true });
      await page.waitForTimeout(300);
      await page.getByRole('button', { name: /save/i }).click();

      await expect(page.getByText('Confirm Network Configuration Change')).toBeVisible({ timeout: 5000 });
      await page.getByRole('checkbox', { name: /Enable automatic rollback/i }).uncheck();
      await page.getByRole('button', { name: /apply changes/i }).click();
      await page.waitForTimeout(500);
      expect(harness.getRollbackState().enabled).toBe(false);
    });
  });

  test.describe('MEDIUM: Form Interactions and Validation', () => {
    test('DNS multiline textarea parsing and submission', async ({ page }) => {
      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: '192.168.1.200', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      const dnsInput = page.getByRole('textbox', { name: /DNS/i }).first();
      await dnsInput.fill('8.8.8.8\n1.1.1.1\n9.9.9.9');

      await harness.saveAndVerify(page);
    });

    test('gateway multiline textarea parsing and submission', async ({ page }) => {
      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: '192.168.1.200', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      const gatewayInput = page.getByRole('textbox', { name: /Gateway/i }).first();
      await gatewayInput.fill('192.168.1.1\n192.168.1.2');

      await harness.saveAndVerify(page);
    });

    test('gateway field readonly when DHCP enabled', async ({ page }) => {
      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: '192.168.1.200', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      await expect(page.getByLabel('Static')).toBeChecked();
      const gatewayField = page.getByRole('textbox', { name: /Gateways/i });
      await expect(gatewayField).toBeEditable();

      await page.getByLabel('DHCP').click({ force: true });
      await page.waitForTimeout(300);
      await expect(page.getByLabel('DHCP')).toBeChecked();
      await expect(gatewayField).not.toBeEditable();

      await page.getByLabel('Static').click({ force: true });
      await page.waitForTimeout(300);
      await expect(page.getByLabel('Static')).toBeChecked();
      await expect(gatewayField).toBeEditable();
    });

    test('netmask dropdown selection', async ({ page }) => {
      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: '192.168.1.200', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      await expect(page.getByText('/24')).toBeVisible();
      await page.getByRole('button', { name: /\/24/i }).click();
      await page.waitForSelector('.v-list-item');
      await page.locator('.v-list-item-title').filter({ hasText: '/16' }).click();

      await expect(page.getByRole('button', { name: /\/16/i })).toBeVisible();
      await expect(page.getByRole('button', { name: /save/i })).toBeEnabled();
    });

    test('form dirty flag tracking', async ({ page }) => {
      await harness.setup(page, {}); // default

      const ipInput = page.getByRole('textbox', { name: /IP Address/i }).first();
      await ipInput.fill('192.168.1.210');

      await expect(page.getByRole('button', { name: /save/i })).toBeEnabled();
      await expect(page.getByRole('button', { name: /reset/i })).toBeEnabled();
    });

    test('form reset button discards unsaved changes', async ({ page }) => {
      const originalIp = '192.168.1.100';
      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: originalIp, dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      const ipInput = page.getByRole('textbox', { name: /IP Address/i }).first();
      await expect(ipInput).toHaveValue(originalIp);

      await ipInput.fill('192.168.1.210');
      await expect(ipInput).toHaveValue('192.168.1.210');

      await page.getByRole('button', { name: /reset/i }).click();
      await expect(ipInput).toHaveValue(originalIp);
    });

    test('tab switching with unsaved changes - discard and switch', async ({ page }) => {
      await harness.setup(page, [
        { name: 'eth0', ipv4: { addrs: [{ addr: '192.168.1.100', dhcp: false, prefix_len: 24 }] } },
        { name: 'eth1', mac: '00:11:22:33:44:56', ipv4: { addrs: [{ addr: '192.168.1.101', dhcp: false, prefix_len: 24 }] } }
      ], 'eth0');

      const ipInput = page.getByRole('textbox', { name: /IP Address/i }).first();
      await ipInput.fill('192.168.1.210');
      await page.waitForTimeout(500);

      await page.getByRole('tab', { name: 'eth1' }).click();
      await expect(page.getByText('Unsaved Changes', { exact: true })).toBeVisible({ timeout: 5000 });

      await page.getByRole('button', { name: /discard/i }).click();
      await page.waitForTimeout(500);
      await expect(page.getByRole('textbox', { name: /IP Address/i }).first()).toBeVisible();
    });

    test('tab switching with unsaved changes - cancel and stay', async ({ page }) => {
      await harness.setup(page, [
        { name: 'eth0', ipv4: { addrs: [{ addr: '192.168.1.100', dhcp: false, prefix_len: 24 }] } },
        { name: 'eth1', mac: '00:11:22:33:44:56', ipv4: { addrs: [{ addr: '192.168.1.101', dhcp: false, prefix_len: 24 }] } }
      ], 'eth0');

      const ipInput = page.getByRole('textbox', { name: /IP Address/i }).first();
      await ipInput.fill('192.168.1.210');
      await page.waitForTimeout(500);

      await page.getByRole('tab', { name: 'eth1' }).click();
      await expect(page.getByText('Unsaved Changes', { exact: true })).toBeVisible({ timeout: 5000 });

      await page.getByRole('button', { name: /cancel/i }).click();
      await page.waitForTimeout(300);

      await expect(ipInput).toHaveValue('192.168.1.210');
      await expect(page.getByText('Unsaved Changes', { exact: true })).not.toBeVisible();
    });
  });

  test.describe('LOW: Edge Cases and UI Polish', () => {
    test('copy to clipboard - IP address', async ({ page, context }) => {
      await context.grantPermissions(['clipboard-read', 'clipboard-write']);
      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: '192.168.1.100', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      await page.locator('.mdi-content-copy').first().click();
      const clipboardText = await page.evaluate(() => navigator.clipboard.readText());
      expect(clipboardText).toMatch(/^[a-zA-Z0-9.:]+\/\d+$/);
    });

    test('copy to clipboard - MAC address', async ({ page, context }) => {
      await context.grantPermissions(['clipboard-read', 'clipboard-write']);
      const testMac = '00:11:22:33:44:55';
      await harness.setup(page, { mac: testMac });

      await page.locator('.mdi-content-copy').nth(1).click();
      const clipboardText = await page.evaluate(() => navigator.clipboard.readText());
      expect(clipboardText).toBe(testMac);
    });

    test('offline adapter handling and display', async ({ page }) => {
      await harness.setup(page, {
        online: false,
        ipv4: {
          addrs: [{ addr: '192.168.1.100', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      await expect(page.locator('.v-chip').filter({ hasText: 'Offline' })).toBeVisible({ timeout: 5000 });
      const ipInput = page.getByRole('textbox', { name: /IP Address/i }).first();
      await expect(ipInput).toBeEditable();
    });

    test('WebSocket sync during editing - dirty flag prevents overwrite', async ({ page }) => {
      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: '192.168.1.100', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      const ipInput = page.getByRole('textbox', { name: /IP Address/i }).first();
      await expect(ipInput).toBeVisible();

      const editedIp = '10.20.30.40';
      await ipInput.fill(editedIp);
      await page.waitForTimeout(500);

      await harness.publishNetworkStatus([
        harness.createAdapter('eth0', {
          ipv4: {
            addrs: [{ addr: '192.168.1.150', dhcp: false, prefix_len: 24 }],
            dns: ['8.8.8.8'],
            gateways: ['192.168.1.1'],
          },
        }),
      ]);

      await page.waitForTimeout(1000);
      await expect(ipInput).toHaveValue(editedIp);
    });

    test('multiple adapters navigation', async ({ page }) => {
      await harness.setup(page, [
        { name: 'eth0', ipv4: { addrs: [{ addr: '10.0.0.1', dhcp: false, prefix_len: 24 }] } },
        { name: 'eth1', mac: '00:11:22:33:44:56', ipv4: { addrs: [{ addr: '10.0.0.2', dhcp: false, prefix_len: 24 }] } }
      ], 'eth0');

      await expect(page.getByRole('tab', { name: 'eth0' })).toBeVisible();
      await expect(page.getByRole('tab', { name: 'eth1' })).toBeVisible();

      await harness.navigateToAdapter(page, 'eth1');
      await expect(page.getByRole('textbox', { name: /IP Address/i }).first()).toBeVisible();
    });

    test('current connection detection - IP match', async ({ page }) => {
      await harness.setup(page, {
        ipv4: {
          addrs: [{ addr: 'localhost', dhcp: false, prefix_len: 24 }],
          dns: ['8.8.8.8'],
          gateways: ['192.168.1.1'],
        },
      });

      await expect(page.getByText('(current connection)')).toBeVisible();
    });

    test('current connection detection - hostname not matching any IP', async ({ page }) => {
      await harness.setup(page, [
        { name: 'eth0', ipv4: { addrs: [{ addr: '192.168.1.100', dhcp: false, prefix_len: 24 }] } },
        { name: 'eth1', mac: '00:11:22:33:44:56', ipv4: { addrs: [{ addr: '192.168.1.101', dhcp: false, prefix_len: 24 }] } }
      ], 'eth0');

      await expect(page.getByText('(current connection)')).not.toBeVisible();

      await harness.navigateToAdapter(page, 'eth1');
      await expect(page.getByText('(current connection)')).not.toBeVisible();
    });
  });
});
