/**
 * Timer management for device operations and network changes
 *
 * This module handles:
 * - Reconnection polling after reboot/factory reset
 * - New IP polling after network config changes
 * - Automatic timeout handling
 */

import { watch } from 'vue'
import { viewModel, isInitialized, wasmModule } from './state'
import type { Event } from '../../../../shared_types/generated/typescript/types/shared_types'
import {
	EventVariantDevice,
	DeviceEventVariantReconnectionCheckTick,
	DeviceEventVariantReconnectionTimeout,
	DeviceEventVariantNewIpCheckTick,
	DeviceEventVariantNewIpCheckTimeout,
} from '../../../../shared_types/generated/typescript/types/shared_types'

// Timer callback type - will be set by index.ts to avoid circular dependency
let sendEventCallback: ((event: Event) => Promise<void>) | null = null

/**
 * Set the event sender callback (called from index.ts after initialization)
 */
export function setEventSender(callback: (event: Event) => Promise<void>): void {
	sendEventCallback = callback
}

// ============================================================================
// Timer Constants
// ============================================================================

const RECONNECTION_POLL_INTERVAL_MS = Number(import.meta.env.VITE_RECONNECTION_POLL_INTERVAL_MS) || 5000 // 5 seconds
const NEW_IP_POLL_INTERVAL_MS = Number(import.meta.env.VITE_NEW_IP_POLL_INTERVAL_MS) || 5000 // 5 seconds

// Optional test overrides for reconnection timeouts (production values come from Core)
const REBOOT_TIMEOUT_OVERRIDE_MS = import.meta.env.VITE_REBOOT_TIMEOUT_MS ? Number(import.meta.env.VITE_REBOOT_TIMEOUT_MS) : null
const FACTORY_RESET_TIMEOUT_OVERRIDE_MS = import.meta.env.VITE_FACTORY_RESET_TIMEOUT_MS ? Number(import.meta.env.VITE_FACTORY_RESET_TIMEOUT_MS) : null

// ============================================================================
// Timer IDs
// ============================================================================

let reconnectionIntervalId: ReturnType<typeof setInterval> | null = null
let reconnectionTimeoutId: ReturnType<typeof setTimeout> | null = null
let reconnectionCountdownIntervalId: ReturnType<typeof setInterval> | null = null
let reconnectionCountdownDeadline: number | null = null
let newIpIntervalId: ReturnType<typeof setInterval> | null = null
let newIpTimeoutId: ReturnType<typeof setTimeout> | null = null
let newIpCountdownIntervalId: ReturnType<typeof setInterval> | null = null

// Countdown deadline for network changes (Unix timestamp in milliseconds)
let countdownDeadline: number | null = null

// ============================================================================
// Reconnection Polling
// ============================================================================

/**
 * Start reconnection polling for reboot/factory reset/update
 * Reads timeout from Core's overlay spinner countdown_seconds.
 * Sends ReconnectionCheckTick every 5 seconds and sets a timeout with countdown.
 */
export function startReconnectionPolling(): void {
	// Read timeout from Core's overlay spinner countdown BEFORE clearing (stop clears countdownSeconds)
	const coreCountdownSeconds = viewModel.overlaySpinner.countdownSeconds

	stopReconnectionPolling() // Clear any existing timers
	if (!coreCountdownSeconds || coreCountdownSeconds <= 0) {
		console.warn('[useCore] startReconnectionPolling: no countdown from Core, skipping timeout')
		return
	}

	// Allow test env override for shorter timeouts
	const isFactoryReset = viewModel.deviceOperationState.type === 'factoryResetting'
	const overrideMs = isFactoryReset ? FACTORY_RESET_TIMEOUT_OVERRIDE_MS : REBOOT_TIMEOUT_OVERRIDE_MS
	const timeoutMs = overrideMs ?? coreCountdownSeconds * 1000
	const countdownSeconds = Math.ceil(timeoutMs / 1000)
	// Update the displayed countdown to match effective timeout
	viewModel.overlaySpinner.countdownSeconds = countdownSeconds
	console.log(`[useCore] Starting reconnection polling (timeout: ${countdownSeconds}s)`)

	// Set countdown deadline
	reconnectionCountdownDeadline = Date.now() + timeoutMs

	// Start polling interval
	reconnectionIntervalId = setInterval(() => {
		if (isInitialized.value && wasmModule.value && sendEventCallback) {
			sendEventCallback(new EventVariantDevice(new DeviceEventVariantReconnectionCheckTick()))
		}
	}, RECONNECTION_POLL_INTERVAL_MS)

	// Start countdown interval (1 second for UI countdown)
	reconnectionCountdownIntervalId = setInterval(() => {
		if (reconnectionCountdownDeadline !== null) {
			const remainingMs = Math.max(0, reconnectionCountdownDeadline - Date.now())
			const remainingSeconds = Math.ceil(remainingMs / 1000)
			viewModel.overlaySpinner.countdownSeconds = remainingSeconds
		}
	}, 1000)

	// Set timeout
	reconnectionTimeoutId = setTimeout(() => {
		console.log('[useCore] Reconnection timeout reached')
		if (isInitialized.value && wasmModule.value && sendEventCallback) {
			sendEventCallback(new EventVariantDevice(new DeviceEventVariantReconnectionTimeout()))
		}
		stopReconnectionPolling()
	}, timeoutMs)
}

/**
 * Stop reconnection polling
 */
export function stopReconnectionPolling(): void {
	if (reconnectionIntervalId !== null) {
		clearInterval(reconnectionIntervalId)
		reconnectionIntervalId = null
	}
	if (reconnectionTimeoutId !== null) {
		clearTimeout(reconnectionTimeoutId)
		reconnectionTimeoutId = null
	}
	if (reconnectionCountdownIntervalId !== null) {
		clearInterval(reconnectionCountdownIntervalId)
		reconnectionCountdownIntervalId = null
	}
	reconnectionCountdownDeadline = null
	viewModel.overlaySpinner.countdownSeconds = null
}

// ============================================================================
// New IP Polling - LocalStorage Persistence
// ============================================================================

const NETWORK_CHANGE_STORAGE_KEY = 'omnect-network-change-state'

interface StoredNetworkChangeState {
	targetIp: string
	deadline: number // Unix timestamp in milliseconds
	rollbackTimeoutSeconds: number
}

/**
 * Save network change state to localStorage
 */
function saveNetworkChangeState(targetIp: string, rollbackTimeoutSeconds: number): void {
	const deadline = Date.now() + rollbackTimeoutSeconds * 1000
	const state: StoredNetworkChangeState = {
		targetIp,
		deadline,
		rollbackTimeoutSeconds,
	}
	try {
		localStorage.setItem(NETWORK_CHANGE_STORAGE_KEY, JSON.stringify(state))
		console.log('[useCore] Saved network change state to localStorage:', state)
	} catch (e) {
		console.error('[useCore] Failed to save network change state to localStorage:', e)
	}
}

/**
 * Load network change state from localStorage
 */
function loadNetworkChangeState(): StoredNetworkChangeState | null {
	try {
		const stored = localStorage.getItem(NETWORK_CHANGE_STORAGE_KEY)
		if (!stored) return null

		const state: StoredNetworkChangeState = JSON.parse(stored)
		console.log('[useCore] Loaded network change state from localStorage:', state)
		return state
	} catch (e) {
		console.error('[useCore] Failed to load network change state from localStorage:', e)
		return null
	}
}

/**
 * Clear network change state from localStorage
 */
function clearNetworkChangeState(): void {
	try {
		localStorage.removeItem(NETWORK_CHANGE_STORAGE_KEY)
		console.log('[useCore] Cleared network change state from localStorage')
	} catch (e) {
		console.error('[useCore] Failed to clear network change state from localStorage:', e)
	}
}

/**
 * Check for pending network change on app initialization
 * Resumes polling if deadline hasn't passed, otherwise checks rollback status
 */
export function checkPendingNetworkChange(): void {
	const stored = loadNetworkChangeState()
	if (!stored) return

	const now = Date.now()
	const timeRemaining = stored.deadline - now

	if (timeRemaining > 0) {
		// Deadline hasn't passed yet - resume polling
		console.log(`[useCore] Resuming network change polling (${Math.round(timeRemaining / 1000)}s remaining)`)
	} else {
		// Deadline has passed - check rollback status and clean up
		console.log('[useCore] Network change deadline has passed, checking rollback status')
		clearNetworkChangeState()
	}
}

// ============================================================================
// New IP Polling
// ============================================================================

/**
 * Start new IP polling after network config change
 * Sends NewIpCheckTick every 5 seconds and sets a timeout based on the backend's rollback timeout
 */
export function startNewIpPolling(): void {
	stopNewIpPolling() // Clear any existing timers

	console.log('[useCore] Starting new IP polling')

	// Clear messages when starting polling so that arriving at new IP/re-login 
	// doesn't have stale success/error state
	viewModel.successMessage = null
	viewModel.errorMessage = null

	// Get timeout from viewModel (provided by backend)
	const state = viewModel.networkChangeState
	if (!state || (state.type !== 'waitingForNewIp' && state.type !== 'waitingForOldIp')) {
		console.warn('[useCore] startNewIpPolling called but state is not waitingForNewIp or waitingForOldIp:', state)
		return
	}

	let rollbackTimeout = 0
	let targetIp = ''
	let switchingToDhcp = false

	if (state.type === 'waitingForNewIp') {
		// Type casting for properties that exist on specific variants
		const s = state as any
		rollbackTimeout = s.rollbackTimeoutSeconds
		targetIp = s.newIp
		switchingToDhcp = s.switchingToDhcp
	} else {
		// waitingForOldIp
		const s = state as any
		// No rollback timeout in this state (we are already rolled back)
		rollbackTimeout = 0
		targetIp = s.oldIp
		// We are polling the old IP, so we know it
		switchingToDhcp = false
	}

	const timeoutMs = rollbackTimeout * 1000

	// Save to localStorage for page refresh resilience
	// For waitingForOldIp, we might not need to save timeout, or save 0
	saveNetworkChangeState(targetIp, rollbackTimeout)

	// Set countdown deadline
	countdownDeadline = Date.now() + timeoutMs

	// Start polling interval (every 5 seconds) ONLY if we are not switching to DHCP
	// If switching to DHCP, we don't know the IP so polling is useless
	if (!switchingToDhcp) {
		newIpIntervalId = setInterval(() => {
			if (isInitialized.value && wasmModule.value && sendEventCallback) {
				sendEventCallback(new EventVariantDevice(new DeviceEventVariantNewIpCheckTick()))
			}
		}, NEW_IP_POLL_INTERVAL_MS)
	} else {
		console.log('[useCore] Skipping polling because switching to DHCP (IP unknown)')
	}

	// Only start countdown and timeout if rollback is enabled (timeout > 0)
	if (rollbackTimeout > 0) {
		// Update countdown immediately
		if (countdownDeadline !== null) {
			const remainingMs = Math.max(0, countdownDeadline - Date.now())
			const remainingSeconds = Math.ceil(remainingMs / 1000)
			viewModel.overlaySpinner.countdownSeconds = remainingSeconds
		}

		// Start countdown interval (every 1 second for UI countdown)
		newIpCountdownIntervalId = setInterval(() => {
			if (countdownDeadline !== null) {
				const remainingMs = Math.max(0, countdownDeadline - Date.now())
				const remainingSeconds = Math.ceil(remainingMs / 1000)
				viewModel.overlaySpinner.countdownSeconds = remainingSeconds
			}
		}, 1000)

		// Set timeout
		newIpTimeoutId = setTimeout(() => {
			console.log('[useCore] New IP polling timeout reached')
			if (isInitialized.value && wasmModule.value && sendEventCallback) {
				sendEventCallback(new EventVariantDevice(new DeviceEventVariantNewIpCheckTimeout()))
			}
			stopNewIpPolling()
		}, timeoutMs)
	}
}

/**
 * Stop new IP polling
 */
export function stopNewIpPolling(): void {
	if (newIpIntervalId !== null) {
		clearInterval(newIpIntervalId)
		newIpIntervalId = null
	}
	if (newIpCountdownIntervalId !== null) {
		clearInterval(newIpCountdownIntervalId)
		newIpCountdownIntervalId = null
	}
	if (newIpTimeoutId !== null) {
		clearTimeout(newIpTimeoutId)
		newIpTimeoutId = null
	}
	// Clear countdown seconds in viewModel
	viewModel.overlaySpinner.countdownSeconds = null
	// Clear countdown deadline
	countdownDeadline = null
}

// ============================================================================
// State Watchers
// ============================================================================

/**
 * Initialize watchers for automatic timer management
 * Call this once during module initialization
 */
export function initializeTimerWatchers(): void {
	// Watch deviceOperationState for reconnection polling
	watch(
		() => viewModel.deviceOperationState,
		(newState, oldState) => {
			const newType = newState?.type
			const oldType = oldState?.type

			// Only act on type transitions
			if (newType === oldType) return

			// Start polling when entering rebooting, factoryResetting, or updating state
			if (newType === 'rebooting' || newType === 'factoryResetting' || newType === 'updating') {
				startReconnectionPolling()
			}
			// Stop polling when leaving these states or entering terminal states
			else if (
				(oldType === 'rebooting' || oldType === 'factoryResetting' || oldType === 'updating' || oldType === 'waitingReconnection') &&
				(newType === 'idle' || newType === 'reconnectionSuccessful' || newType === 'reconnectionFailed')
			) {
				stopReconnectionPolling()
			}
		},
		{ deep: true }
	)

	// Watch networkChangeState for new IP polling and redirect
	watch(
		() => viewModel.networkChangeState,
		(newState, oldState) => {
			const newType = newState?.type
			const oldType = oldState?.type

			// Start polling when entering polling states
			const isPollingState = (type: string | undefined) =>
				type === 'waitingForNewIp' || type === 'waitingForOldIp'

			if (isPollingState(newType) && !isPollingState(oldType)) {
				startNewIpPolling()
			}
			// Stop polling when leaving polling states
			else if (isPollingState(oldType) && !isPollingState(newType)) {
				stopNewIpPolling()
			}
			// If switching between polling states (e.g. newIp -> oldIp), restart to update config/target
			else if (isPollingState(newType) && isPollingState(oldType) && newType !== oldType) {
				startNewIpPolling()
			}

			// Clear localStorage when entering terminal states (success, timeout, or idle)
			if (
				newType !== oldType &&
				(newType === 'newIpReachable' || newType === 'newIpTimeout' || newType === 'idle')
			) {
				clearNetworkChangeState()
			}

			// Navigate to new IP when it's reachable
			if (newState?.type === 'newIpReachable') {
				console.log(`[useCore] Redirecting to new IP: ${newState.newIp}:${newState.uiPort}`)
				// Clear messages before redirecting so they don't persist on arrival at new IP
				viewModel.successMessage = null
				viewModel.errorMessage = null
				// Use HTTPS (server only listens on HTTPS)
				window.location.href = `https://${newState.newIp}:${newState.uiPort}`
			}
		},
		{ deep: true }
	)
}