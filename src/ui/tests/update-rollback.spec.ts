import { test, expect } from '@playwright/test';
import { setupAndLogin } from './fixtures/test-setup';
import { publishToCentrifugo } from './fixtures/centrifugo';

const mockUpdateEndpoints = async (page: import('@playwright/test').Page, version = '4.0.24') => {
  await page.route('**/update/file', route => route.fulfill({ status: 200, body: 'OK' }));
  await page.route('**/update/load', route =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        updateId: { provider: 'omnect', name: 'gateway', version },
        isDeployable: true,
        compatibility: [],
        createdDateTime: '2024-01-19T12:00:00Z',
        manifestVersion: '1.0',
      }),
    })
  );
};

const uploadAndInstall = async (page: import('@playwright/test').Page, version: string) => {
  let runCalled = false;
  await page.route('**/update/run', async route => {
    runCalled = true;
    await route.fulfill({ status: 200, body: '' });
  });
  await page.locator('input[type="file"]').setInputFiles({
    name: 'update.swu',
    mimeType: 'application/x-tar',
    buffer: Buffer.from('dummy'),
  });
  await expect(page.getByText(version)).toBeVisible({ timeout: 15000 });
  await page.getByRole('button', { name: 'Install Update' }).click();
  await expect(async () => expect(runCalled).toBe(true)).toPass();
};

const mockRollbackHealthcheck = async (
  page: import('@playwright/test').Page,
  abortCount = 2
) => {
  let attempts = 0;
  await page.route('**/healthcheck', async route => {
    attempts++;
    if (attempts <= abortCount) {
      await route.abort('failed');
    } else {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          versionInfo: { current: '4.0.0', required: '4.0.0', mismatch: false },
          updateValidationStatus: { status: 'Recovered' },
          networkRollbackOccurred: false,
          updateValidationAcked: false,
        }),
      });
    }
  });
};

test.use({ viewport: { width: 1440, height: 900 } });

test.describe('Device Update Rollback', () => {
  test.beforeEach(async ({ page }) => {
    page.on('console', msg => console.log(`BROWSER CONSOLE: ${msg.type()} - ${msg.text()}`));
    
    // Reset Centrifugo state from previous tests to prevent modals from appearing on login
    await publishToCentrifugo('UpdateValidationStatusV1', { status: 'NoUpdate' });

    await setupAndLogin(page, { updateValidationAcked: false });

    // Navigate to update page
    await expect(page.locator('[data-cy="main-nav"]')).toBeVisible();
    await page.locator('[data-cy="main-nav"]').getByText('Update').click();
    await expect(page).toHaveURL(/.*\/update/);
    await expect(page.getByText('Update Details', { exact: true })).toBeVisible();
  });

  test('shows rollback notification after failed update', async ({ page }) => {
    // 2. Mock Update Process
    let uploadCalled = false;
    await page.route('**/update/file', async (route) => {
      uploadCalled = true;
      await route.fulfill({ status: 200, body: 'OK' });
    });

    const mockManifest = {
        updateId: {
            provider: "omnect",
            name: "gateway",
            version: "4.0.24"
        },
        isDeployable: true,
        compatibility: [
            {
                manufacturer: "omnect",
                model: "rpi4",
                compatibilityid: "rpi4"
            }
        ],
        createdDateTime: "2024-01-19T12:00:00Z",
        manifestVersion: "1.0"
    };

    let loadCalled = false;
    await page.route('**/update/load', async (route) => {
        loadCalled = true;
        await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(mockManifest) });
    });

    let runCalled = false;
    await page.route('**/update/run', async (route) => {
        runCalled = true;
        await route.fulfill({ status: 200, body: '' });
    });

    // Perform Upload
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles({
        name: 'test-update.swu',
        mimeType: 'application/x-tar',
        buffer: Buffer.from('dummy content')
    });

    // Verify mocks were hit
    await expect(async () => expect(uploadCalled).toBe(true)).toPass();
    await expect(async () => expect(loadCalled).toBe(true)).toPass();

    // Wait for manifest to load
    await expect(page.getByText('4.0.24')).toBeVisible({ timeout: 15000 });

    // 3. Trigger Update
    const installBtn = page.getByRole('button', { name: 'Install Update' });
    await installBtn.click();

    await expect(async () => expect(runCalled).toBe(true)).toPass();
    await expect(page.getByText('Update installed, initiating reboot...')).toBeVisible();

    // 4. Mock Healthcheck Rollback
    // - Simulate offline (abort)
    // - Then simulate back online with status: 'Recovered' and ack: false
    let healthcheckAttempts = 0;
    await page.route('**/healthcheck', async (route) => {
        healthcheckAttempts++;
        if (healthcheckAttempts <= 2) {
            await route.abort('failed');
        } else {
            await route.fulfill({
                status: 200,
                contentType: 'application/json',
                body: JSON.stringify({
                    versionInfo: { current: '4.0.0', required: '4.0.0', mismatch: false }, // Back to old version
                    updateValidationStatus: { status: 'Recovered' },
                    networkRollbackOccurred: false,
                    updateValidationAcked: false
                })
            });
        }
    });

    // 5. Verify Rollback Modal
    // The UI should redirect to login because the update is "done" (even if recovered)
    await expect(page).toHaveURL(/.*\/login/, { timeout: 25000 });

    // Assert that the modal is NOT visible before login
    await expect(page.getByText('Update Rolled Back')).not.toBeVisible();
    await expect(page.getByText('Update Succeeded')).not.toBeVisible();

    // Login again to establish WebSocket connection
    await page.getByPlaceholder(/enter your password/i).fill('password');
    await page.getByRole('button', { name: /log in/i }).click();
    await expect(page.getByText('Common Info')).toBeVisible();

    // Simulate WebSocket message with update validation result arriving
    await publishToCentrifugo('UpdateValidationStatusV1', { status: 'Recovered' });

    // The modal should be visible in App.vue
    const rollbackModal = page.getByText('Update Rolled Back');
    await expect(rollbackModal).toBeVisible({ timeout: 15000 });
    await expect(page.getByText('The firmware update could not be validated. The previous version has been restored.')).toBeVisible();

    // 6. Verify Acknowledgment
    let ackCalled = false;
    await page.route('**/ack-update-validation', async (route) => {
        ackCalled = true;
        await route.fulfill({ status: 200, body: '' });
    });

    await page.getByRole('button', { name: 'OK' }).click();

    // Modal should disappear
    await expect(rollbackModal).not.toBeVisible();

    // Ack should have been called
    await expect(async () => expect(ackCalled).toBe(true)).toPass();

    // Update page must be fresh — Core cleared the manifest on the Recovered healthcheck
    await page.locator('[data-cy="main-nav"]').getByText('Update').click();
    await expect(page).toHaveURL(/.*\/update/);
    await expect(page.getByText('4.0.24')).not.toBeVisible();

    // Regression: uploading a new file after rollback must not throw a TypeError.
    // The watcher on deviceOperationState previously raced with an in-flight axios POST,
    // clearing updateFile.value mid-request so that file.name access after `await` threw
    // "Cannot read properties of undefined (reading 'name')".
    uploadCalled = false;
    await page.locator('input[type="file"]').setInputFiles({
      name: 'second-update.swu',
      mimeType: 'application/x-tar',
      buffer: Buffer.from('dummy'),
    });
    await expect(async () => expect(uploadCalled).toBe(true)).toPass({ timeout: 5000 });
    await expect(page.getByText(/uploading file failed/i)).not.toBeVisible();
    // update/load is called only when UploadCompleted fires with a valid file.name — if a
    // TypeError had been thrown, UploadCompleted would be absent and the manifest would not load.
    await expect(page.getByText('4.0.24')).toBeVisible({ timeout: 10000 });
  });

  test('shows timed-out state when device does not come back online', async ({ page }) => {
    // Mock update endpoints
    await page.route('**/update/file', async (route) => {
      await route.fulfill({ status: 200, body: 'OK' });
    });

    await page.route('**/update/load', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          updateId: { provider: 'omnect', name: 'gateway', version: '4.0.24' },
          isDeployable: true,
          compatibility: [],
          createdDateTime: '2024-01-19T12:00:00Z',
          manifestVersion: '1.0'
        })
      });
    });

    let runCalled = false;
    await page.route('**/update/run', async (route) => {
      runCalled = true;
      await route.fulfill({ status: 200, body: '' });
    });

    // Upload file
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles({
      name: 'test-update.swu',
      mimeType: 'application/x-tar',
      buffer: Buffer.from('dummy content')
    });
    await expect(page.getByText('4.0.24')).toBeVisible({ timeout: 15000 });

    // Override healthcheck to always abort — device never comes back online
    await page.route('**/healthcheck', async (route) => {
      await route.abort('failed');
    });

    // Install update
    await page.getByRole('button', { name: 'Install Update' }).click();
    await expect(async () => expect(runCalled).toBe(true)).toPass();

    // Overlay should appear with spinner while device is rebooting
    await expect(page.getByText('Rebooting to new firmware')).toBeVisible({ timeout: 10000 });
    await expect(page.locator('#overlay .v-progress-circular')).toBeVisible();
    await expect(page.locator('#overlay .mdi-alert-circle-outline')).not.toBeVisible();

    // After VITE_FIRMWARE_UPDATE_TIMEOUT_MS (2s in test build), spinner is replaced by warning icon
    await expect(page.locator('#overlay .mdi-alert-circle-outline')).toBeVisible({ timeout: 10000 });
    await expect(page.locator('#overlay .v-progress-circular')).not.toBeVisible();

    // Countdown must not be shown in the timed-out state
    await expect(page.getByText('Time remaining:')).not.toBeVisible();
    await expect(page.getByText('Timeout in:')).not.toBeVisible();

    // Both action buttons must be present
    await expect(page.getByRole('button', { name: /open app in new tab/i })).toBeVisible();
    await expect(page.getByRole('button', { name: /refresh/i })).toBeVisible();
  });

  test('shows second rollback modal in same SPA session after acking first', async ({ page }) => {
    // This test covers the race where updateValidationAckedOnMount goes stale:
    // 1. First rollback modal appears and is acked → updateValidationAckedOnMount = true
    // 2. Second update runs → reconnection polling sets viewModel.healthcheck.updateValidationAcked = false
    // 3. SPA re-login (no page reload) → onMounted never re-runs, flag stays stale
    // 4. Second Recovered arrives via WebSocket → combined watcher uses live healthcheck value → modal shown

    page.on('console', msg => console.log(`BROWSER CONSOLE: ${msg.type()} - ${msg.text()}`));

    // Stage first rollback in Centrifugo history so it replays immediately on subscription
    await publishToCentrifugo('UpdateValidationStatusV1', { status: 'Recovered' });

    // Login with updateValidationAcked: false — first rollback not yet acked
    await setupAndLogin(page, { updateValidationAcked: false });

    // First rollback modal must appear
    const firstModal = page.getByText('Update Rolled Back');
    await expect(firstModal).toBeVisible({ timeout: 15000 });

    // Ack the first modal — internally sets updateValidationAckedOnMount = true
    let firstAckCalled = false;
    await page.route('**/ack-update-validation', async route => {
      firstAckCalled = true;
      await route.fulfill({ status: 200, body: '' });
    });
    await page.getByRole('button', { name: 'OK' }).click();
    await expect(firstModal).not.toBeVisible();
    await expect(async () => expect(firstAckCalled).toBe(true)).toPass();

    // Navigate to update page for the second update
    await page.locator('[data-cy="main-nav"]').getByText('Update').click();
    await expect(page).toHaveURL(/.*\/update/);
    await expect(page.getByText('Update Details', { exact: true })).toBeVisible();

    // Mock update endpoints
    await mockUpdateEndpoints(page, '4.1.0');

    // Mock healthcheck: device goes offline (2 aborts) then returns Recovered + updateValidationAcked: false.
    // The successful response sets viewModel.healthcheck.updateValidationAcked = false in Core,
    // which overrides the stale updateValidationAckedOnMount = true on the second SPA login.
    await mockRollbackHealthcheck(page, 2);

    // Upload and install the second update
    await uploadAndInstall(page, '4.1.0');
    await expect(page.getByText('Update installed, initiating reboot...')).toBeVisible();

    // Device reboots — app redirects to login once reconnection polling detects the result
    await expect(page).toHaveURL(/.*\/login/, { timeout: 25000 });

    // SPA re-login: onMounted does NOT re-run so updateValidationAckedOnMount stays true (stale).
    // viewModel.healthcheck.updateValidationAcked = false from the reconnection polling above.
    await page.getByPlaceholder(/enter your password/i).fill('password');
    await page.getByRole('button', { name: /log in/i }).click();
    await expect(page.getByText('Common Info')).toBeVisible();

    // Centrifugo delivers Recovered. Combined watcher: ackedInHealthcheck=false → notAcked=true → modal shown
    await publishToCentrifugo('UpdateValidationStatusV1', { status: 'Recovered' });

    const secondModal = page.getByText('Update Rolled Back');
    await expect(secondModal).toBeVisible({ timeout: 15000 });
    await expect(page.getByText('The firmware update could not be validated. The previous version has been restored.')).toBeVisible();

    // Ack the second modal
    let secondAckCalled = false;
    await page.route('**/ack-update-validation', async route => {
      secondAckCalled = true;
      await route.fulfill({ status: 200, body: '' });
    });
    await page.getByRole('button', { name: 'OK' }).click();
    await expect(secondModal).not.toBeVisible();
    await expect(async () => expect(secondAckCalled).toBe(true)).toPass();
  });

  test('shows update success notification', async ({ page }) => {
    // Mock Update Process
    let uploadCalled = false;
    await page.route('**/update/file', async (route) => { 
        uploadCalled = true;
        await route.fulfill({ status: 200, body: 'OK' }); 
    });
    
    let loadCalled = false;
    await page.route('**/update/load', async (route) => { 
        loadCalled = true;
        await route.fulfill({ 
            status: 200, 
            contentType: 'application/json', 
            body: JSON.stringify({
                updateId: { provider: "omnect", name: "gateway", version: "4.0.24" },
                isDeployable: true,
                compatibility: [],
                createdDateTime: "2024-01-19T12:00:00Z",
                manifestVersion: "1.0"
            }) 
        }); 
    });

    let runCalled = false;
    await page.route('**/update/run', async (route) => { 
        runCalled = true;
        await route.fulfill({ status: 200, body: '' }); 
    });

    // Perform Upload
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles({ name: 'test.swu', mimeType: 'application/x-tar', buffer: Buffer.from('x') });
    
    // Verify mocks were hit
    await expect(async () => expect(uploadCalled).toBe(true)).toPass();
    await expect(async () => expect(loadCalled).toBe(true)).toPass();

    await expect(page.getByText('4.0.24')).toBeVisible({ timeout: 15000 });

    // Trigger Update
    await page.getByRole('button', { name: 'Install Update' }).click();
    await expect(async () => expect(runCalled).toBe(true)).toPass();

    // Mock Healthcheck Success
    let healthcheckAttempts = 0;
    await page.route('**/healthcheck', async (route) => {
        healthcheckAttempts++;
        if (healthcheckAttempts <= 1) {
            await route.abort('failed');
        } else {
            await route.fulfill({
                status: 200,
                contentType: 'application/json',
                body: JSON.stringify({
                    versionInfo: { current: '4.0.24', required: '4.0.24', mismatch: false },
                    updateValidationStatus: { status: 'Succeeded' },
                    networkRollbackOccurred: false,
                    updateValidationAcked: false
                })
            });
        }
    });

    // Login after success
    await expect(page).toHaveURL(/.*\/login/, { timeout: 25000 });

    // Assert that the modal is NOT visible before login
    await expect(page.getByText('Update Rolled Back')).not.toBeVisible();
    await expect(page.getByText('Update Succeeded')).not.toBeVisible();

    await page.getByPlaceholder(/enter your password/i).fill('password');
    await page.getByRole('button', { name: /log in/i }).click();
    await expect(page.getByText('Common Info')).toBeVisible();

    // Simulate WebSocket message
    await publishToCentrifugo('UpdateValidationStatusV1', { status: 'Succeeded' });

    // Verify Success Modal
    await expect(page.getByText('Update Succeeded')).toBeVisible({ timeout: 15000 });
    await expect(page.getByText('The firmware update was applied and validated successfully.')).toBeVisible();

    // Acknowledge
    await page.getByRole('button', { name: 'OK' }).click();
    await expect(page.getByText('Update Succeeded')).not.toBeVisible();
  });
});
