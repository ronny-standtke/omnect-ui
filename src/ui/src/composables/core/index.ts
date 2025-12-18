/**
 * Vue composable for integrating with Crux Core
 *
 * This composable provides the bridge between the Vue UI shell and the
 * Crux Core (Rust compiled to WASM). It handles:
 * - Sending events to the core
 * - Processing effects from the core
 * - Providing reactive access to the view model
 *
 * The generated types from shared_types provide serialization/deserialization
 * for bincode FFI communication with the WASM module.
 *
 * Build the WASM module with:
 *   cd src/app && wasm-pack build --target web --out-dir ../ui/src/core/pkg
 *
 * Generate TypeScript types with:
 *   export PATH="$HOME/.local/share/pnpm:$PATH" && cargo build -p shared_types
 */

import { readonly, type DeepReadonly } from 'vue'

// Import state
import {
	viewModel,
	isInitialized,
	isSubscribed,
	authToken,
	wasmModule,
	initializationPromise,
	setWasmModule,
	setInitializationPromise,
} from './state'

// Import effects processing
import { processEffects } from './effects'

// Import timer management
import { setEventSender as setTimerEventSender, initializeTimerWatchers, checkPendingNetworkChange } from './timers'

// Import Centrifugo
import { setEventSender as setCentrifugoEventSender } from './centrifugo'

// Import sync
import { setEventSender as setSyncEventSender } from './sync'

// Import serialization
import { BincodeSerializer } from '../../../../shared_types/generated/typescript/bincode/mod'

// Import event types
import type { Event } from '../../../../shared_types/generated/typescript/types/shared_types'
import {
	EventVariantInitialize,
	EventVariantAuth,
	EventVariantDevice,
	EventVariantWebSocket,
	EventVariantUi,
	AuthEventVariantLogin,
	AuthEventVariantLogout,
	AuthEventVariantSetPassword,
	AuthEventVariantUpdatePassword,
	AuthEventVariantCheckRequiresPasswordSet,
	DeviceEventVariantReboot,
	DeviceEventVariantFactoryResetRequest,
	DeviceEventVariantSetNetworkConfig,
	DeviceEventVariantLoadUpdate,
	DeviceEventVariantRunUpdate,
	DeviceEventVariantNetworkFormStartEdit,
	DeviceEventVariantNetworkFormUpdate,
	DeviceEventVariantNetworkFormReset,
	DeviceEventVariantAckRollback,
	WebSocketEventVariantSubscribeToChannels,
	WebSocketEventVariantUnsubscribeFromChannels,
	UiEventVariantClearError,
	UiEventVariantClearSuccess,
} from '../../../../shared_types/generated/typescript/types/shared_types'

// Re-export types for external use
export type {
	ViewModel,
	DeviceOperationStateType,
	NetworkChangeStateType,
	NetworkFormStateType,
	NetworkFormDataType,
	OverlaySpinnerStateType,
	FactoryResetStatusString,
	SystemInfo,
	NetworkStatus,
	OnlineStatus,
	FactoryReset,
	UpdateValidationStatus,
	Timeouts,
	HealthcheckInfo,
	Event,
	Effect,
	CoreViewModel,
	UpdateManifest,
	NetworkFormData,
} from './types'

// ============================================================================
// Event Serialization
// ============================================================================

/**
 * Serialize an Event to bincode bytes for sending to WASM Core
 */
function serializeEvent(event: Event): Uint8Array {
	const serializer = new BincodeSerializer()
	event.serialize(serializer)
	return serializer.getBytes()
}

// ============================================================================
// Core Communication
// ============================================================================

/**
 * Send an event to the Crux Core
 *
 * This serializes the event, sends it to the WASM core, and processes
 * any resulting effects.
 */
async function sendEventToCore(event: Event): Promise<void> {
	if (!isInitialized.value || !wasmModule) {
		console.warn('Core not initialized, cannot send event')
		return
	}

	try {
		// Serialize the event using bincode
		const eventBytes = serializeEvent(event)

		// Call process_event() on the WASM module
		const effectsBytes = wasmModule.process_event(eventBytes) as Uint8Array

		// Process the resulting effects
		await processEffects(effectsBytes)
	} catch (error) {
		console.error('Failed to send event to core:', error)
	}
}

// Wire up event sender callbacks to break circular dependencies
setTimerEventSender(sendEventToCore)
setCentrifugoEventSender(sendEventToCore)
setSyncEventSender(sendEventToCore)

// Initialize timer watchers
initializeTimerWatchers()

// ============================================================================
// Initialization
// ============================================================================

/**
 * Initialize the Crux Core
 *
 * This loads the WASM module and sets up the core state.
 * Uses a promise-based guard to prevent both race conditions and premature event sending.
 */
async function initializeCore(): Promise<void> {
	// If initialization is already in progress or complete, wait for/return that promise
	if (initializationPromise) {
		return initializationPromise
	}

	// Create and store the initialization promise
	const promise = (async () => {
		console.log('Initializing Crux Core...')

		try {
			// Dynamically import the WASM module
			// This will be available after running:
			// cd src/app && wasm-pack build --target web --out-dir ../ui/src/core/pkg
			const wasm = await import('../../core/pkg/omnect_ui_core')
			await wasm.default()
			setWasmModule(wasm)

			console.log('Crux Core WASM module loaded successfully')

			// Only set initialized flag after WASM is fully loaded
			isInitialized.value = true

			// Check for pending network change from previous session
			checkPendingNetworkChange()

			// Send initial event
			await sendEventToCore(new EventVariantInitialize())
		} catch (error) {
			console.error('Failed to load Crux Core WASM module:', error)
			console.log('Running in fallback mode without WASM')
			// Set initialized flag even on error to prevent retry loops
			isInitialized.value = true
		}
	})()

	setInitializationPromise(promise)
	return promise
}

// ============================================================================
// Public API
// ============================================================================

/**
 * Vue composable for Crux Core integration
 *
 * Provides the main interface for Vue components to interact with the Rust
 * Crux Core compiled to WASM.
 *
 * @example
 * ```typescript
 * const { viewModel, sendEvent, initialize } = useCore()
 *
 * onMounted(async () => {
 *   await initialize()
 * })
 *
 * // Access reactive view model
 * const isOnline = computed(() => viewModel.online_status?.iothub ?? false)
 *
 * // Send events using convenience methods
 * login('password')
 * reboot()
 * ```
 */
export function useCore() {
	return {
		// Provide readonly access to the view model
		viewModel: readonly(viewModel) as DeepReadonly<typeof viewModel>,

		// Event sending (using Event type from shared_types)
		sendEvent: sendEventToCore,
		// Provide authToken for direct use
		authToken: readonly(authToken),

		// Initialization
		initialize: initializeCore,
		isInitialized: readonly(isInitialized),

		// Convenience methods for common events
		login: (password: string) => sendEventToCore(new EventVariantAuth(new AuthEventVariantLogin(password))),
		logout: () => sendEventToCore(new EventVariantAuth(new AuthEventVariantLogout())),
		setPassword: (password: string) =>
			sendEventToCore(new EventVariantAuth(new AuthEventVariantSetPassword(password))),
		updatePassword: (currentPassword: string, password: string) =>
			sendEventToCore(new EventVariantAuth(new AuthEventVariantUpdatePassword(currentPassword, password))),
		checkRequiresPasswordSet: () =>
			sendEventToCore(new EventVariantAuth(new AuthEventVariantCheckRequiresPasswordSet())),
		reboot: () => sendEventToCore(new EventVariantDevice(new DeviceEventVariantReboot())),
		factoryReset: (mode: string, preserve: string[]) =>
			sendEventToCore(new EventVariantDevice(new DeviceEventVariantFactoryResetRequest(mode, preserve))),
		setNetworkConfig: (config: string) =>
			sendEventToCore(new EventVariantDevice(new DeviceEventVariantSetNetworkConfig(config))),
		loadUpdate: (filePath: string) =>
			sendEventToCore(new EventVariantDevice(new DeviceEventVariantLoadUpdate(filePath))),
		runUpdate: (validateIothub: boolean) =>
			sendEventToCore(new EventVariantDevice(new DeviceEventVariantRunUpdate(validateIothub))),
		subscribeToChannels: () => {
			if (isSubscribed.value) {
				return
			}
			if (!authToken.value) {
				console.warn('[useCore] Skipping subscription: no auth token')
				return
			}
			isSubscribed.value = true
			sendEventToCore(new EventVariantWebSocket(new WebSocketEventVariantSubscribeToChannels()))
		},
		unsubscribeFromChannels: () => {
			isSubscribed.value = false
			sendEventToCore(new EventVariantWebSocket(new WebSocketEventVariantUnsubscribeFromChannels()))
		},
		clearError: () => sendEventToCore(new EventVariantUi(new UiEventVariantClearError())),
		clearSuccess: () => sendEventToCore(new EventVariantUi(new UiEventVariantClearSuccess())),

		// Network form state management
		networkFormStartEdit: (adapterName: string) =>
			sendEventToCore(new EventVariantDevice(new DeviceEventVariantNetworkFormStartEdit(adapterName))),
		networkFormUpdate: (formDataJson: string) =>
			sendEventToCore(new EventVariantDevice(new DeviceEventVariantNetworkFormUpdate(formDataJson))),
		networkFormReset: (adapterName: string) =>
			sendEventToCore(new EventVariantDevice(new DeviceEventVariantNetworkFormReset(adapterName))),
		ackRollback: () =>
			sendEventToCore(new EventVariantDevice(new DeviceEventVariantAckRollback())),
	}
}
