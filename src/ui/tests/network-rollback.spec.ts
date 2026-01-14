import { test, expect } from '@playwright/test';
import { mockConfig, mockLoginSuccess, mockRequireSetPassword } from './fixtures/mock-api';
import { publishToCentrifugo } from './fixtures/centrifugo';

test.describe('Network Rollback Status', () => {
  test('rollback status is cleared after ack and does not reappear on re-login', async ({ page, context }) => {
    // Track healthcheck calls
    let healthcheckRollbackStatus = true;

    await mockConfig(page);
    await mockLoginSuccess(page);
    await mockRequireSetPassword(page);

    // Mock healthcheck with rollback occurred status
    await page.route('**/healthcheck', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          version_info: {
            required: '>=0.39.0',
            current: '0.40.0',
            mismatch: false,
          },
          update_validation_status: {
            status: 'valid',
          },
          network_rollback_occurred: healthcheckRollbackStatus,
        }),
      });
    });

    // Mock ack-rollback endpoint
    await page.route('**/ack-rollback', async (route) => {
      if (route.request().method() === 'POST') {
        // Simulate clearing the rollback status on the backend
        healthcheckRollbackStatus = false;
        await route.fulfill({
          status: 200,
        });
      }
    });

    // Step 1: Navigate to page - rollback notification appears on mount (before login)
    await page.goto('/');

    // The rollback notification dialog appears immediately (from healthcheck in onMounted)
    await expect(page.getByText('Network Settings Rolled Back')).toBeVisible({ timeout: 10000 });

    // Step 2: Acknowledge the rollback message
    // This should call /ack-rollback (now without auth requirement) and clear the backend marker
    await page.getByRole('button', { name: /ok/i }).click();
    await expect(page.getByText('Network Settings Rolled Back')).not.toBeVisible();

    // Wait a moment for the async POST to /ack-rollback to complete
    await page.waitForTimeout(500);

    // Now we can log in
    await page.getByPlaceholder(/enter your password/i).fill('password');
    await page.getByRole('button', { name: /log in/i }).click();
    await expect(page.getByText('Common Info')).toBeVisible({ timeout: 10000 });

    // Step 3: Reload the page to simulate logout and re-login
    await page.reload();

    // The rollback notification should NOT appear again because we acknowledged it
    // and the /ack-rollback call cleared the backend marker file
    await expect(page.getByText('Network Settings Rolled Back')).not.toBeVisible({ timeout: 3000 });

    // Can proceed with login
    await page.getByPlaceholder(/enter your password/i).fill('password');
    await page.getByRole('button', { name: /log in/i }).click();
    await expect(page.getByText('Common Info')).toBeVisible({ timeout: 10000 });

    // Verify no rollback notification
    await expect(page.getByText('Network Settings Rolled Back')).not.toBeVisible();
  });
});
