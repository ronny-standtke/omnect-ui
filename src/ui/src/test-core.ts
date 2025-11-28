/**
 * Test script for Crux Core integration
 *
 * This file provides helper functions to test the Core/Shell integration
 * directly from the browser console. Import this in main.ts or use it
 * directly in the console.
 *
 * Usage (in browser console):
 *   1. Import the test functions
 *   2. Run testLogin() to test the full HTTP flow
 *   3. Check console logs for effect processing details
 */

import { useCore } from './composables/useCore'

// Get Core instance
const { viewModel, initialize, login, logout, reboot, clearError, clearSuccess, isInitialized } =
  useCore()

/**
 * Test Core initialization
 */
export async function testInit(): Promise<void> {
  console.log('=== Testing Core Initialization ===')
  console.log('Current state:', {
    isInitialized: isInitialized.value,
    viewModel: { ...viewModel },
  })

  if (!isInitialized.value) {
    console.log('Initializing Core...')
    await initialize()
    console.log('Core initialized')
  }

  console.log('ViewModel after init:', { ...viewModel })
}

/**
 * Test login flow - this will test the full HTTP effect pipeline
 *
 * @param username - Username to test with (default: "admin")
 * @param password - Password to test with (default: "test")
 */
export async function testLogin(username = 'admin', password = 'test'): Promise<void> {
  console.log('=== Testing Login Flow ===')
  console.log(`Attempting login with username: ${username}`)

  // Ensure Core is initialized
  if (!isInitialized.value) {
    await initialize()
  }

  console.log('Before login:', {
    is_loading: viewModel.is_loading,
    is_authenticated: viewModel.is_authenticated,
    error_message: viewModel.error_message,
  })

  // Trigger login
  await login(username, password)

  // Note: The login request is async, so the response will come later
  console.log('After login event sent:', {
    is_loading: viewModel.is_loading,
    is_authenticated: viewModel.is_authenticated,
    error_message: viewModel.error_message,
  })

  // Wait a bit for the HTTP request to complete
  setTimeout(() => {
    console.log('After HTTP response (1s later):', {
      is_loading: viewModel.is_loading,
      is_authenticated: viewModel.is_authenticated,
      error_message: viewModel.error_message,
    })
  }, 1000)
}

/**
 * Test logout flow
 */
export async function testLogout(): Promise<void> {
  console.log('=== Testing Logout Flow ===')

  if (!isInitialized.value) {
    await initialize()
  }

  console.log('Before logout:', {
    is_authenticated: viewModel.is_authenticated,
  })

  await logout()

  console.log('After logout event sent:', {
    is_loading: viewModel.is_loading,
  })
}

/**
 * Test reboot request (protected endpoint)
 */
export async function testReboot(): Promise<void> {
  console.log('=== Testing Reboot Request ===')

  if (!isInitialized.value) {
    await initialize()
  }

  if (!viewModel.is_authenticated) {
    console.warn('Not authenticated - reboot will likely fail')
  }

  await reboot()

  console.log('Reboot event sent, check logs for HTTP effect')
}

/**
 * Clear any error messages
 */
export async function testClearError(): Promise<void> {
  console.log('=== Clearing Error Message ===')
  await clearError()
  console.log('Error message:', viewModel.error_message)
}

/**
 * Clear any success messages
 */
export async function testClearSuccess(): Promise<void> {
  console.log('=== Clearing Success Message ===')
  await clearSuccess()
  console.log('Success message:', viewModel.success_message)
}

/**
 * Get current viewModel state
 */
export function getState(): object {
  return {
    system_info: viewModel.system_info,
    network_status: viewModel.network_status,
    online_status: viewModel.online_status,
    factory_reset: viewModel.factory_reset,
    update_validation_status: viewModel.update_validation_status,
    timeouts: viewModel.timeouts,
    healthcheck: viewModel.healthcheck,
    is_authenticated: viewModel.is_authenticated,
    requires_password_set: viewModel.requires_password_set,
    is_loading: viewModel.is_loading,
    error_message: viewModel.error_message,
    success_message: viewModel.success_message,
    is_connected: viewModel.is_connected,
  }
}

/**
 * Watch viewModel changes
 */
export function watchState(): void {
  console.log('Current state:', getState())
  console.log('Use getState() to check state at any time')
}

// Export for easy console access
if (typeof window !== 'undefined') {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  ;(window as any).coreTest = {
    testInit,
    testLogin,
    testLogout,
    testReboot,
    testClearError,
    testClearSuccess,
    getState,
    watchState,
    viewModel,
    isInitialized,
  }
  console.log(
    'Core test functions available at window.coreTest',
    '\nTry: coreTest.testInit() or coreTest.testLogin()'
  )
}
