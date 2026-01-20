import { test, expect } from '@playwright/test';
import { setupAndLogin } from './fixtures/test-setup';

test.describe('Device Reboot', () => {
  test.beforeEach(async ({ page }) => {
    await setupAndLogin(page);
  });

  test('user can initiate reboot from the device actions menu', async ({ page }) => {
    // Mock the reboot endpoint
    let rebootCalled = false;
    await page.route('**/reboot', async (route) => {
      rebootCalled = true;
      await route.fulfill({ status: 200, body: '' });
    });

    // Locate and click the Reboot button (it's in DeviceActions)
    // The main button is the one with text "Reboot"
    const rebootBtn = page.getByRole('button', { name: 'Reboot' }).first(); 
    await rebootBtn.click();

    // Verify dialog appears
    await expect(page.getByText('Reboot device', { exact: true })).toBeVisible();
    await expect(page.getByText('Do you really want to restart the device?')).toBeVisible();

    // Click Reboot in the dialog
    // We target the dialog specifically to be safe
    const dialog = page.getByRole('dialog');
    const confirmBtn = dialog.getByRole('button', { name: 'Reboot' });
    await confirmBtn.click();

    // Verify API call
    // Wait for a short moment to ensure the click processed and request fired
    await page.waitForTimeout(100);
    expect(rebootCalled).toBe(true);

    // Verify UI feedback (OverlaySpinner)
    // The spinner text should change to "Device is rebooting"
    // Use a longer timeout for this expectation as it depends on WASM processing the effect response
    await expect(page.getByText('Device is rebooting')).toBeVisible({ timeout: 10000 });
  });

  test('user can cancel the reboot dialog', async ({ page }) => {
    // Open dialog
    const rebootBtn = page.getByRole('button', { name: 'Reboot' }).first(); 
    await rebootBtn.click();

    // Verify dialog appears
    await expect(page.getByText('Reboot device', { exact: true })).toBeVisible();

    // Click Cancel
    const cancelBtn = page.getByRole('button', { name: 'Cancel' });
    await cancelBtn.click();

    // Verify dialog disappears
    await expect(page.getByText('Reboot device', { exact: true })).not.toBeVisible();
    
    // Verify API was NOT called (we can't easily verify "not called" without a spy, 
    // but we can ensure no visual feedback of rebooting)
    await expect(page.getByText('Device is rebooting')).not.toBeVisible();
  });

  test('displays timeout message when device does not come back online', async ({ page }) => {
    // Mock the reboot endpoint
    await page.route('**/reboot', async (route) => {
      await route.fulfill({ status: 200, body: '' });
    });

    // Mock healthcheck to ALWAYS fail (simulating offline device)
    await page.route('**/healthcheck', async (route) => {
      // Force connection error or simply don't fulfill to simulate timeout/unreachable
      // But re-connection polling expects a response (even error) to count attempts?
      // Actually, if we just abort, it might look like network error.
      await route.abort('failed');
    });

    // Initiate reboot
    const rebootBtn = page.getByRole('button', { name: 'Reboot' }).first(); 
    await rebootBtn.click();
    
    const confirmBtn = page.getByRole('dialog').getByRole('button', { name: 'Reboot' });
    await confirmBtn.click();

    // Verify initial state
    await expect(page.getByText('Device is rebooting')).toBeVisible();

    // Wait for timeout (configured to 2000ms in test env)
    // We add a little buffer to be safe
    await page.waitForTimeout(2500);

    // Verify timeout message
    // The exact text comes from Rust: 
    // "Device did not come back online after 5 minutes. Please check the device manually."
    await expect(page.getByText('Device did not come back online after 5 minutes')).toBeVisible();
  });
});
