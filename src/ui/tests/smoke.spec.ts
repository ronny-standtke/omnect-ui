import { test, expect } from '@playwright/test';
import { mockLoginSuccess, mockRequireSetPassword, mockConfig } from './fixtures/mock-api';

test('has title', async ({ page }) => {
  await mockConfig(page);
  await page.goto('/');

  // Expect a title "to contain" a substring.
  await expect(page).toHaveTitle(/omnect/i);
});

test('login flow', async ({ page }) => {
  // Listen for console logs
  page.on('console', msg => console.log(`BROWSER LOG: ${msg.text()}`));
  page.on('pageerror', err => console.log(`BROWSER ERROR: ${err}`));

  await mockConfig(page);
  await mockLoginSuccess(page);
  await mockRequireSetPassword(page);
  
  await page.goto('/');
  
  // Wait for the form to appear (it's hidden while checking password set requirement)
  // Using placeholder as Vuetify labels can be tricky with getByLabel
  await expect(page.getByPlaceholder(/enter your password/i)).toBeVisible({ timeout: 10000 });
  
  // Check for the login button
  await expect(page.getByRole('button', { name: /log in/i })).toBeVisible();
});
