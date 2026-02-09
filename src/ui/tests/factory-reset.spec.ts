import { test, expect } from '@playwright/test';
import { setupAndLogin } from './fixtures/test-setup';

test.describe('Device Factory Reset', () => {
  test.beforeEach(async ({ page }) => {
    await setupAndLogin(page);
  });

  test('user can initiate factory reset from the device actions menu', async ({ page }) => {
    // Mock the factory-reset endpoint
    let resetCalled = false;
    await page.route('**/factory-reset', async (route) => {
      resetCalled = true;
      const request = route.request();
      const postData = await request.postDataJSON();
      // Verify payload
      expect(postData.mode).toBe(1);
      expect(postData.preserve).toEqual([]);
      await route.fulfill({ status: 200, body: '' });
    });

    // Locate and click the Factory Reset button (it's in DeviceActions)
    const resetBtn = page.getByRole('button', { name: 'Factory Reset' }).first(); 
    await resetBtn.click();

    // Verify dialog appears
    await expect(page.getByText('Factory reset', { exact: true })).toBeVisible();
    
    // Click Reset in the dialog
    const dialog = page.getByRole('dialog');
    const confirmBtn = dialog.getByRole('button', { name: 'Reset' });
    await confirmBtn.click();

    // Verify API call
    await page.waitForTimeout(100);
    expect(resetCalled).toBe(true);

    // Verify UI feedback
    await expect(page.getByText('The device is resetting')).toBeVisible({ timeout: 10000 });
  });

  test('user can cancel the factory reset dialog', async ({ page }) => {
    // Open dialog
    const resetBtn = page.getByRole('button', { name: 'Factory Reset' }).first(); 
    await resetBtn.click();

    // Verify dialog appears
    await expect(page.getByText('Factory reset', { exact: true })).toBeVisible();

    // Click Cancel
    const cancelBtn = page.getByRole('button', { name: 'Cancel' });
    await cancelBtn.click();

    // Verify dialog disappears
    await expect(page.getByText('Factory reset', { exact: true })).not.toBeVisible();
    
    await expect(page.getByText('The device is resetting')).not.toBeVisible();
  });

  test('displays timeout message when device does not come back online', async ({ page }) => {
    // Mock the factory-reset endpoint
    await page.route('**/factory-reset', async (route) => {
      await route.fulfill({ status: 200, body: '' });
    });

    // Mock healthcheck to ALWAYS fail (simulating offline device)
    await page.route('**/healthcheck', async (route) => {
      await route.abort('failed');
    });

    // Initiate factory reset
    const resetBtn = page.getByRole('button', { name: 'Factory Reset' }).first(); 
    await resetBtn.click();
    
    const confirmBtn = page.getByRole('dialog').getByRole('button', { name: 'Reset' });
    await confirmBtn.click();

    // Verify initial state
    await expect(page.getByText('The device is resetting')).toBeVisible();

    // Wait for timeout
    // Wait for timeout (countdown comes from Core's overlay spinner)
    await page.waitForTimeout(1000);

    // Verify timeout message
    await expect(page.getByText('Device did not come back online. You may need to re-accept the security certificate.')).toBeVisible();
  });
});
