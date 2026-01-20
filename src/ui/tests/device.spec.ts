import { test, expect } from '@playwright/test';
import { publishToCentrifugo } from './fixtures/centrifugo';
import { setupAndLogin } from './fixtures/test-setup';

test.describe('Device Info', () => {
  test.beforeEach(async ({ page }) => {
    // Listen for console logs
    page.on('console', msg => console.log(`BROWSER LOG: ${msg.text()}`));
    page.on('pageerror', err => console.log(`BROWSER ERROR: ${err}`));

    await setupAndLogin(page);
  });

  test('displays system info from Centrifugo', async ({ page }) => {
    const systemInfo = {
      os: {
        name: 'Omnect OS',
        version: '1.2.3',
      },
      azure_sdk_version: '0.1.0',
      omnect_device_service_version: '4.5.6',
      boot_time: new Date().toISOString(),
    };

    // Publish to Centrifugo
    await publishToCentrifugo('SystemInfoV1', systemInfo);

    // Assert values appear on dashboard
    // Adjust selectors based on actual UI
    await expect(page.getByText('Omnect OS')).toBeVisible();
    await expect(page.getByText('1.2.3')).toBeVisible();
    await expect(page.getByText('4.5.6')).toBeVisible();
  });
});
