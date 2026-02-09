/**
 * ViewModel synchronization for Crux Core
 *
 * This module handles syncing the reactive Vue viewModel with
 * the Crux Core's serialized state.
 */

import { viewModel, authToken, isSubscribed, wasmModule, centrifugoInstance } from './state'
import {
	factoryResetStatusToString,
	convertDeviceOperationState,
	convertNetworkChangeState,
	convertNetworkFormState,
	convertUploadState,
} from './types'
import { setViewModelUpdater } from './effects'
import { Model as GeneratedViewModel } from '../../../../shared_types/generated/typescript/types/shared_types'
import { BincodeDeserializer } from '../../../../shared_types/generated/typescript/bincode/mod'
import type { Event } from '../../../../shared_types/generated/typescript/types/shared_types'
import {
	EventVariantWebSocket,
	WebSocketEventVariantSubscribeToChannels,
} from '../../../../shared_types/generated/typescript/types/shared_types'

// Event sender callback - set by index.ts to avoid circular dependency
let sendEventCallback: ((event: Event) => Promise<void>) | null = null

/**
 * Set the event sender callback (called from index.ts after initialization)
 */
export function setEventSender(callback: (event: Event) => Promise<void>): void {
	sendEventCallback = callback
}

/**
 * Fetch and deserialize the view model from Crux Core
 *
 * Reads the serialized viewModel bytes from WASM, deserializes using bincode,
 * and updates the reactive Vue viewModel object.
 */
export function updateViewModelFromCore(): void {
	if (!wasmModule.value) {
		return
	}

	try {
		// Capture authentication state before update to detect transitions
		const wasAuthenticated = viewModel.isAuthenticated

		// Get serialized view model from WASM
		const viewModelBytes = wasmModule.value.view() as Uint8Array

		// Deserialize it using the generated ViewModel class
		const deserializer = new BincodeDeserializer(viewModelBytes)
		const coreViewModel = GeneratedViewModel.deserialize(deserializer)

		// Update the reactive view model with deserialized data
		// systemInfo
		if (coreViewModel.systemInfo) {
			viewModel.systemInfo = {
				os: {
					name: coreViewModel.systemInfo.os.name,
					version: coreViewModel.systemInfo.os.version,
				},
				azureSdkVersion: coreViewModel.systemInfo.azureSdkVersion,
				omnectDeviceServiceVersion: coreViewModel.systemInfo.omnectDeviceServiceVersion,
				bootTime: coreViewModel.systemInfo.bootTime || null,
			}
		} else {
			viewModel.systemInfo = null
		}

		// networkStatus
		if (coreViewModel.networkStatus) {
			viewModel.networkStatus = {
				networkStatus: coreViewModel.networkStatus.networkStatus,
			}
		} else {
			viewModel.networkStatus = null
		}

		// onlineStatus
		viewModel.onlineStatus = coreViewModel.onlineStatus ? { iothub: coreViewModel.onlineStatus.iothub } : null

		// factoryReset - convert status variant to string literal
		try {
			viewModel.factoryReset = coreViewModel.factoryReset
				? {
						keys: coreViewModel.factoryReset.keys,
						result: coreViewModel.factoryReset.result
							? {
									status: factoryResetStatusToString(coreViewModel.factoryReset.result.status),
									context: coreViewModel.factoryReset.result.context || null,
									error: coreViewModel.factoryReset.result.error,
									paths: coreViewModel.factoryReset.result.paths,
								}
							: null,
					}
				: null
		} catch (error) {
			console.warn('[updateViewModelFromCore] Failed to deserialize factoryReset, keeping existing value:', error)
		}

		// updateValidationStatus
		viewModel.updateValidationStatus = coreViewModel.updateValidationStatus
			? { status: coreViewModel.updateValidationStatus.status }
			: null

		// updateManifest
		viewModel.updateManifest = coreViewModel.updateManifest ?? null

		// timeouts
		viewModel.timeouts = coreViewModel.timeouts
			? {
					waitOnlineTimeout: {
						nanos: coreViewModel.timeouts.waitOnlineTimeout.nanos,
						secs: coreViewModel.timeouts.waitOnlineTimeout.secs,
					},
				}
			: null

		// healthcheck
		viewModel.healthcheck = coreViewModel.healthcheck
			? {
					versionInfo: {
						required: coreViewModel.healthcheck.versionInfo.required,
						current: coreViewModel.healthcheck.versionInfo.current,
						mismatch: coreViewModel.healthcheck.versionInfo.mismatch,
					},
					updateValidationStatus: {
						status: coreViewModel.healthcheck.updateValidationStatus.status,
					},
					networkRollbackOccurred: coreViewModel.healthcheck.networkRollbackOccurred,
				}
			: null

		// Boolean and string fields
		viewModel.isAuthenticated = coreViewModel.isAuthenticated
		viewModel.requiresPasswordSet = coreViewModel.requiresPasswordSet
		viewModel.isLoading = coreViewModel.isLoading
		viewModel.errorMessage = coreViewModel.errorMessage || null
		viewModel.successMessage = coreViewModel.successMessage || null
		viewModel.isConnected = coreViewModel.isConnected
		viewModel.authToken = coreViewModel.authToken || null

		// Sync the ref with the view model
		authToken.value = viewModel.authToken

		// Overlay spinner state (synced BEFORE device/network state so watchers can read countdown)
		// Preserve countdownSeconds if it's being actively managed by the Shell (countdown interval running)
		const isNetworkChangeActive = viewModel.networkChangeState.type === 'waitingForNewIp'
		const isDeviceOpActive = viewModel.deviceOperationState.type === 'rebooting'
			|| viewModel.deviceOperationState.type === 'factoryResetting'
			|| viewModel.deviceOperationState.type === 'updating'
			|| viewModel.deviceOperationState.type === 'waitingReconnection'
		const preserveCountdown = (isNetworkChangeActive || isDeviceOpActive)
			&& viewModel.overlaySpinner.countdownSeconds !== null

		viewModel.overlaySpinner = {
			overlay: coreViewModel.overlaySpinner.overlay,
			title: coreViewModel.overlaySpinner.title,
			text: coreViewModel.overlaySpinner.text || null,
			timedOut: coreViewModel.overlaySpinner.timedOut,
			progress: coreViewModel.overlaySpinner.progress !== null && coreViewModel.overlaySpinner.progress !== undefined
				? coreViewModel.overlaySpinner.progress
				: null,
			countdownSeconds: preserveCountdown
				? viewModel.overlaySpinner.countdownSeconds // Keep Shell's calculated value
				: (coreViewModel.overlaySpinner.countdownSeconds !== null && coreViewModel.overlaySpinner.countdownSeconds !== undefined
					? coreViewModel.overlaySpinner.countdownSeconds
					: null),
		}

		// Device operation state - convert bincode variant to typed object
		viewModel.deviceOperationState = convertDeviceOperationState(coreViewModel.deviceOperationState)
		viewModel.reconnectionAttempt = coreViewModel.reconnectionAttempt

		// Network change state
		viewModel.networkChangeState = convertNetworkChangeState(coreViewModel.networkChangeState)

		// Network form state
		viewModel.networkFormState = convertNetworkFormState(coreViewModel.networkFormState)

		// Network form dirty flag
		viewModel.networkFormDirty = coreViewModel.networkFormDirty

		// Browser hostname and current connection adapter (computed in Core)
		viewModel.browserHostname = coreViewModel.browserHostname || null
		viewModel.currentConnectionAdapter = coreViewModel.currentConnectionAdapter || null

		// Device offline tracking
		viewModel.deviceWentOffline = coreViewModel.deviceWentOffline

		// Network rollback modal state (computed in Core)
		viewModel.shouldShowRollbackModal = coreViewModel.shouldShowRollbackModal
		viewModel.defaultRollbackEnabled = coreViewModel.defaultRollbackEnabled

		// Firmware upload state
		viewModel.firmwareUploadState = convertUploadState(coreViewModel.firmwareUploadState)

		// Auto-subscribe logic based on authentication state transition
		if (viewModel.isAuthenticated && !wasAuthenticated) {
			console.log('[useCore] User authenticated, triggering subscription')
			if (authToken.value && !isSubscribed.value && sendEventCallback) {
				isSubscribed.value = true
				sendEventCallback(new EventVariantWebSocket(new WebSocketEventVariantSubscribeToChannels()))
			}
		}

		// Reset subscription state on logout
		if (!viewModel.isAuthenticated && wasAuthenticated) {
			console.log('[useCore] User logged out, resetting subscription state and disconnecting Centrifugo')
			isSubscribed.value = false
			// Disconnect Centrifugo to ensure old tokens are not reused
			centrifugoInstance.disconnect()
		}
	} catch (error) {
		console.error('Failed to update view model from core:', error)
		// Don't throw - keep the viewModel as-is from events
		// This allows the app to continue working even if Core's viewModel has deserialization issues
	}
}

// Wire up the circular dependency: effects.ts needs to call updateViewModelFromCore
setViewModelUpdater(updateViewModelFromCore)