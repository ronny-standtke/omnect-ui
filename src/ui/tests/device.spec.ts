import { test, expect } from '@playwright/test';
import { mockConfig, mockLoginSuccess, mockRequireSetPassword } from './fixtures/mock-api';
import { publishToCentrifugo } from './fixtures/centrifugo';

test.describe('Device Info', () => {
  test.beforeEach(async ({ page }) => {
    // Listen for console logs
    page.on('console', msg => console.log(`BROWSER LOG: ${msg.text()}`));
    page.on('pageerror', err => console.log(`BROWSER ERROR: ${err}`));

    await mockConfig(page);
    await mockLoginSuccess(page);
    await mockRequireSetPassword(page);
    await page.goto('/');
    
    // Perform login
    await page.getByPlaceholder(/enter your password/i).fill('password');
    await page.getByRole('button', { name: /log in/i }).click();
    
    // Wait for dashboard or successful login state
    // We can wait for the side menu or a specific dashboard element
    await expect(page.getByText('Common Info')).toBeVisible();
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
