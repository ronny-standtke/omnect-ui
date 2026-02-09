/**
 * Singleton reactive state for Crux Core integration
 *
 * This module contains all shared reactive state used by the Core composable.
 * By centralizing state here, we avoid circular dependencies between modules.
 */

import { ref, reactive } from 'vue'
import type { ViewModel } from './types'
import { useCentrifuge } from '../useCentrifugo'

// ============================================================================
// Singleton Reactive State
// ============================================================================

/**
 * The main reactive view model, mirroring the Crux Core's Model struct
 */
export const viewModel = reactive<ViewModel>({
	systemInfo: null,
	networkStatus: null,
	onlineStatus: null,
	factoryReset: null,
	updateValidationStatus: null,
	updateManifest: null,
	timeouts: null,
	healthcheck: null,
	isAuthenticated: false,
	requiresPasswordSet: false,
	isLoading: false,
	errorMessage: null,
	successMessage: null,
	isConnected: false,
	authToken: null,
	// Device operation state
	deviceOperationState: { type: 'idle' },
	reconnectionAttempt: 0,
	// Network change state
	networkChangeState: { type: 'idle' },
	// Network form state
	networkFormState: { type: 'idle' },
	// Network form dirty flag
	networkFormDirty: false,
	// Browser hostname and current connection detection
	browserHostname: null,
	currentConnectionAdapter: null,
	deviceWentOffline: false,
	// Network rollback modal state
	shouldShowRollbackModal: false,
	defaultRollbackEnabled: true,
	// Firmware upload state
	firmwareUploadState: { type: 'idle' },
	// Overlay spinner state
	overlaySpinner: { overlay: false, title: '', text: null, timedOut: false, progress: null, countdownSeconds: null },
})

/**
 * Whether the WASM Core has been initialized
 */
export const isInitialized = ref(false)

/**
 * Whether we're subscribed to Centrifugo channels
 */
export const isSubscribed = ref(false)

/**
 * Auth token ref for direct use
 */
export const authToken = ref<string | null>(null)

/**
 * WASM module reference (set when WASM is loaded)
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const wasmModule = ref<any>(null)

/**
 * Set the WASM module reference
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function setWasmModule(module: any): void {
	wasmModule.value = module
}

/**
 * Promise-based initialization guard to prevent race conditions
 */
export let initializationPromise: Promise<void> | null = null

/**
 * Set the initialization promise
 */
export function setInitializationPromise(promise: Promise<void>): void {
	initializationPromise = promise
}

// ============================================================================
// Centrifugo Instance
// ============================================================================

/**
 * Centrifugo instance for WebSocket operations
 */
export const centrifugoInstance = useCentrifuge()

// Inject the auth token ref into Centrifugo to avoid circular dependency
centrifugoInstance.setAuthToken(authToken)