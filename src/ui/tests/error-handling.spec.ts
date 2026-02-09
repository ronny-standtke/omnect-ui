import { test, expect } from '@playwright/test';
import { setupAndLogin } from './fixtures/test-setup';

test.describe('General Error Handling', () => {
  test.beforeEach(async ({ page }) => {
    await setupAndLogin(page);
  });

  test('displays detailed error message from backend on 500 error', async ({ page }) => {
    const detailedErrorMessage = 'Reboot failed: Downstream service ODS returned a version mismatch error.';
    
    // Mock reboot endpoint to return 500 with a detailed message
    await page.route('**/reboot', async (route) => {
      await route.fulfill({
        status: 500,
        contentType: 'text/plain',
        body: detailedErrorMessage,
      });
    });

    // 1. Open Reboot Dialog
    // DeviceOverview.vue contains DeviceActions.vue
    const rebootActivator = page.getByRole('button', { name: 'Reboot', exact: true });
    await expect(rebootActivator).toBeVisible();
    await rebootActivator.click();

    // 2. Click the confirm Reboot button in the dialog
    // The dialog button also has the name 'Reboot' but it's inside the dialog
    const confirmButton = page.locator('.v-dialog').getByRole('button', { name: 'Reboot', exact: true });
    await expect(confirmButton).toBeVisible();
    await confirmButton.click();

    // 3. Verify error message appears in snackbar
    // useSnackbar.ts sets the message in a global state shown in App.vue
    const snackbar = page.locator('.v-snackbar__content');
    await expect(snackbar).toBeVisible();
    await expect(snackbar).toContainText(detailedErrorMessage);
  });
});
