import { Page, expect } from '@playwright/test';
import { mockConfig, mockLoginSuccess, mockRequireSetPassword } from './mock-api';

export async function setupAndLogin(page: Page) {
  await mockConfig(page);
  await mockLoginSuccess(page);
  await mockRequireSetPassword(page);
  
  // Mock healthcheck to avoid errors on app load
  await page.route('**/healthcheck', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
          version_info: { current: '1.0.0', required: '1.0.0', mismatch: false },
          update_validation_status: { status: 'NoUpdate' },
          network_rollback_occurred: false,
          factory_reset_result_acked: true,
          update_validation_acked: true
      })
    });
  });

  // Mock initial network config to avoid errors
  await page.route('**/network', async (route) => {
      if (route.request().method() === 'GET') {
          await route.fulfill({
              status: 200,
              body: JSON.stringify({ interfaces: [] })
          });
      } else {
          await route.continue();
      }
  });

  await page.goto('/');
  
  // Perform login
  await page.getByPlaceholder(/enter your password/i).fill('password');
  await page.getByRole('button', { name: /log in/i }).click();
  
  // Wait for dashboard
  await expect(page.getByText('Common Info')).toBeVisible();
}
