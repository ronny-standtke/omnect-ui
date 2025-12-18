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
	system_info: null,
	network_status: null,
	online_status: null,
	factory_reset: null,
	update_validation_status: null,
	update_manifest: null,
	timeouts: null,
	healthcheck: null,
	is_authenticated: false,
	requires_password_set: false,
	is_loading: false,
	error_message: null,
	success_message: null,
	is_connected: false,
	auth_token: null,
	// Device operation state
	device_operation_state: { type: 'idle' },
	reconnection_attempt: 0,
	reconnection_timeout_seconds: 300, // 5 minutes default
	// Network change state
	network_change_state: { type: 'idle' },
	// Network form state
	network_form_state: { type: 'idle' },
	// Network form dirty flag
	network_form_dirty: false,
	// Firmware upload state
	firmware_upload_state: { type: 'idle' },
	// Overlay spinner state
	overlay_spinner: { overlay: false, title: '', text: null, timed_out: false, progress: null, countdown_seconds: null },
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
export let wasmModule: any = null

/**
 * Set the WASM module reference
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function setWasmModule(module: any): void {
	wasmModule = module
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
