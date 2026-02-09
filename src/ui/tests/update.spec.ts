import { test, expect } from '@playwright/test';
import { setupAndLogin } from './fixtures/test-setup';

test.use({ viewport: { width: 1440, height: 900 } });

test.describe('Device Update', () => {
  test.beforeEach(async ({ page }) => {
    await setupAndLogin(page);

    // Navigate to update page using sidebar to preserve WASM state
    // BaseSideBar has <v-list ... data-cy="main-nav">
    
    await expect(page.locator('[data-cy="main-nav"]')).toBeVisible();
    await page.locator('[data-cy="main-nav"]').getByText('Update').click();
    
    // Verify we are on update page
    await expect(page).toHaveURL(/.*\/update/);
    // Auto-upload means no upload button initially
    await expect(page.getByText('Update Details', { exact: true })).toBeVisible();
  });

  test('successfully uploads and installs firmware update', async ({ page }) => {
    // 1. Mock Upload
    let uploadCalled = false;
    await page.route('**/update/file', async (route) => {
      uploadCalled = true;
      await route.fulfill({ status: 200, body: 'OK' });
    });

    // 2. Mock Load Update (returns manifest)
    const mockManifest = {
        updateId: {
            provider: "omnect",
            name: "gateway-devel",
            version: "4.0.24"
        },
        isDeployable: true,
        compatibility: [
            {
                manufacturer: "omnect",
                model: "raspberrypi4-64",
                compatibilityid: "raspberrypi4-64"
            }
        ],
        createdDateTime: "2024-01-19T12:00:00Z",
        manifestVersion: "1.0"
    };

    let loadCalled = false;
    await page.route('**/update/load', async (route) => {
        loadCalled = true;
        // The Core sends the path, usually empty string or filename
        await route.fulfill({
            status: 200,
            contentType: 'application/json',
            body: JSON.stringify(mockManifest)
        });
    });

    // 3. Mock Run Update
    let runCalled = false;
    await page.route('**/update/run', async (route) => {
        runCalled = true;
        await route.fulfill({ status: 200, body: '' });
    });

    // 4. Perform Upload
    // We need to bypass the file chooser or use setInputFiles
    // The v-file-upload component might hide the actual input
    // We target the input[type=file]
    const fileInput = page.locator('input[type="file"]');
    
    await fileInput.setInputFiles({
        name: 'test-update.swu',
        mimeType: 'application/x-tar',
        buffer: Buffer.from('dummy content')
    });

    // Auto-upload triggers immediately
    // Verify Upload occurred
    await expect(async () => expect(uploadCalled).toBe(true)).toPass();

    // Verify Load occurred
    await expect(async () => expect(loadCalled).toBe(true)).toPass();

    // Verify Manifest details are displayed
    await expect(page.getByText('4.0.24')).toBeVisible();
    await expect(page.getByText('raspberrypi4-64').first()).toBeVisible();

    // 5. Trigger Update
    // Note: Button text changed to Title Case in redesign
    const installBtn = page.getByRole('button', { name: 'Install Update' });
    await installBtn.click();

    // Verify Run occurred
    await expect(async () => expect(runCalled).toBe(true)).toPass();

    // 6. Verify Update Progress/Completion
    // Core enters 'Updating' state. 
    // We need to simulate the healthcheck flow:
    // - First, it must go offline (simulate reboot)
    // - Then it returns status: 'Succeeded'
    
    let healthcheckAttempts = 0;
    // We override the healthcheck route to simulate offline then success
    await page.route('**/healthcheck', async (route) => {
        healthcheckAttempts++;
        if (healthcheckAttempts <= 2) {
            // Simulate offline
            await route.abort('failed');
        } else {
            // Simulate back online with success
            await route.fulfill({
                status: 200,
                contentType: 'application/json',
                body: JSON.stringify({
                    versionInfo: { current: '4.0.24', required: '4.0.24', mismatch: false }, // Updated version
                    updateValidationStatus: { status: 'Succeeded' },
                    networkRollbackOccurred: false
                })
            });
        }
    });

    // The UI should show success message
    // "Update started" should be visible initially.
    await expect(page.getByText('Update started')).toBeVisible();
    
    // Eventually (after ~1-1.5s), it should succeed and redirect to login
    await expect(page).toHaveURL(/.*\/login/, { timeout: 10000 });
  });
});
