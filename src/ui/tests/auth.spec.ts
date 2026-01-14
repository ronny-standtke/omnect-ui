import { test, expect } from '@playwright/test';
import { mockConfig, mockLoginSuccess, mockRequireSetPassword } from './fixtures/mock-api';

test.describe('Authentication', () => {
  test.beforeEach(async ({ page }) => {
    // Listen for console logs
    page.on('console', msg => console.log(`BROWSER LOG: ${msg.text()}`));
    page.on('pageerror', err => console.log(`BROWSER ERROR: ${err}`));

    await mockConfig(page);
    await mockLoginSuccess(page);
    await mockRequireSetPassword(page);
    
    // Mock logout endpoint
    await page.route('**/logout', async (route) => {
        await route.fulfill({
            status: 200,
            contentType: 'application/json',
            body: JSON.stringify({}),
        });
    });
  });

  test('can logout successfully', async ({ page }) => {
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
});
