import { test, expect } from '@playwright/test';
import {
  mockConfig,
  mockLoginSuccess,
  mockRequireSetPassword,
  mockSetPasswordSuccess,
  mockUpdatePasswordSuccess,
  mockPortalAuth
} from './fixtures/mock-api';

test.describe('Authentication', () => {
  test.beforeEach(async ({ page }) => {
    // Listen for console logs
    page.on('console', msg => console.log(`BROWSER LOG: ${msg.text()}`));
    page.on('pageerror', err => console.log(`BROWSER ERROR: ${err}`));

    await mockConfig(page);
    await mockLoginSuccess(page);

    // Mock logout endpoint
    await page.route('**/logout', async (route) => {
        await route.fulfill({
            status: 200,
            contentType: 'application/json',
            body: JSON.stringify({}),
        });
    });
  });

  test('can login successfully', async ({ page }) => {
    await mockRequireSetPassword(page);
    await page.goto('/');

    // Login
    await page.getByPlaceholder(/enter your password/i).fill('password');
    await page.getByRole('button', { name: /log in/i }).click();

    // Wait for dashboard
    await expect(page.getByText('Common Info')).toBeVisible();
  });

  test('can logout successfully', async ({ page }) => {
    await mockRequireSetPassword(page);
    await page.goto('/');

    // Login
    await page.getByPlaceholder(/enter your password/i).fill('password');
    await page.getByRole('button', { name: /log in/i }).click();

    // Wait for dashboard
    await expect(page.getByText('Common Info')).toBeVisible();

    // Open user menu
    await page.locator('[data-cy="user-menu"]').click();

    // Click logout button
    await page.getByRole('button', { name: /logout/i }).click();

    // Assert redirect to login page
    await expect(page.getByPlaceholder(/enter your password/i)).toBeVisible();
  });

  test('redirects to set-password if required', async ({ page }) => {
    await mockPortalAuth(page);
    // Mock require-set-password returning true
    await page.route('**/require-set-password', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: 'true',
      });
    });

    await page.goto('/');

    // Should be on set-password page
    await expect(page).toHaveURL(/\/set-password/);
    await expect(page.getByText(/set password/i).first()).toBeVisible();
  });

  test('can set initial password successfully', async ({ page }) => {
    await mockPortalAuth(page);
    // Mock require-set-password returning true
    await page.route('**/require-set-password', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: 'true',
      });
    });
    await mockSetPasswordSuccess(page);

    await page.goto('/');

    // Fill set-password form
    // Using nth(0) for first password field as Vuetify labels might match multiple elements
    await page.locator('input[type="password"]').nth(0).fill('new-password');
    await page.locator('input[type="password"]').nth(1).fill('new-password');
    await page.getByRole('button', { name: /set password/i }).click();

    // Should show success message and eventually redirect to dashboard (via auto-login)
    await expect(page.getByText(/password set successfully/i)).toBeVisible();

    // Should be redirected to dashboard
    await expect(page.getByText('Common Info')).toBeVisible();
  });

  test('can update password successfully', async ({ page }) => {
    await mockRequireSetPassword(page);
    await mockUpdatePasswordSuccess(page);
    await page.goto('/');

    // Login first
    await page.getByPlaceholder(/enter your password/i).fill('password');
    await page.getByRole('button', { name: /log in/i }).click();
    await expect(page.getByText('Common Info')).toBeVisible();

    // Navigate to update-password via user menu
    await page.locator('[data-cy="user-menu"]').click();
    await page.getByRole('button', { name: /change password/i }).click();

    // Fill update-password form
    await expect(page.getByText(/update password/i).first()).toBeVisible();

    // Using nth to avoid strict mode violations with Vuetify icons having similar labels/aria-labels
    await page.locator('input[type="password"]').nth(0).fill('password');
    await page.locator('input[type="password"]').nth(1).fill('new-password');
    await page.locator('input[type="password"]').nth(2).fill('new-password');

    await page.getByRole('button', { name: /set new password/i }).click();

    // Verify success message
    await expect(page.getByText(/password updated successfully/i)).toBeVisible();
  });
});