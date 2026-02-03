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
		const wasAuthenticated = viewModel.is_authenticated

		// Get serialized view model from WASM
		const viewModelBytes = wasmModule.value.view() as Uint8Array

		// Deserialize it using the generated ViewModel class
		const deserializer = new BincodeDeserializer(viewModelBytes)
		const coreViewModel = GeneratedViewModel.deserialize(deserializer)

		// Update the reactive view model with deserialized data
		// system_info
		if (coreViewModel.system_info) {
			viewModel.system_info = {
				os: {
					name: coreViewModel.system_info.os.name,
					version: coreViewModel.system_info.os.version,
				},
				azure_sdk_version: coreViewModel.system_info.azure_sdk_version,
				omnect_device_service_version: coreViewModel.system_info.omnect_device_service_version,
				boot_time: coreViewModel.system_info.boot_time || null,
			}
		} else {
			viewModel.system_info = null
		}

		// network_status
		if (coreViewModel.network_status) {
			viewModel.network_status = {
				network_status: coreViewModel.network_status.network_status.map((net) => ({
					ipv4: {
						addrs: net.ipv4.addrs.map((addr) => ({
							addr: addr.addr,
							dhcp: addr.dhcp,
							prefix_len: addr.prefix_len,
						})),
						dns: net.ipv4.dns,
						gateways: net.ipv4.gateways,
					},
					mac: net.mac,
					name: net.name,
					online: net.online,
				})),
			}
		} else {
			viewModel.network_status = null
		}

		// online_status
		viewModel.online_status = coreViewModel.online_status ? { iothub: coreViewModel.online_status.iothub } : null

		// factory_reset - convert status variant to string literal
		// Note: Skip if deserialization fails (can happen with bincode format mismatches)
		try {
			viewModel.factory_reset = coreViewModel.factory_reset
				? {
						keys: coreViewModel.factory_reset.keys,
						result: coreViewModel.factory_reset.result
							? {
									status: factoryResetStatusToString(coreViewModel.factory_reset.result.status),
									context: coreViewModel.factory_reset.result.context || null,
									error: coreViewModel.factory_reset.result.error,
									paths: coreViewModel.factory_reset.result.paths,
								}
							: null,
					}
				: null
		} catch (error) {
			console.warn('[updateViewModelFromCore] Failed to deserialize factory_reset, keeping existing value:', error)
		}

		// update_validation_status
		viewModel.update_validation_status = coreViewModel.update_validation_status
			? { status: coreViewModel.update_validation_status.status }
			: null

		// update_manifest
		viewModel.update_manifest = coreViewModel.update_manifest ?? null

		// timeouts
		viewModel.timeouts = coreViewModel.timeouts
			? {
					wait_online_timeout: {
						nanos: coreViewModel.timeouts.wait_online_timeout.nanos,
						secs: coreViewModel.timeouts.wait_online_timeout.secs,
					},
				}
			: null

		// healthcheck
		viewModel.healthcheck = coreViewModel.healthcheck
			? {
					version_info: {
						required: coreViewModel.healthcheck.version_info.required,
						current: coreViewModel.healthcheck.version_info.current,
						mismatch: coreViewModel.healthcheck.version_info.mismatch,
					},
					update_validation_status: {
						status: coreViewModel.healthcheck.update_validation_status.status,
					},
					network_rollback_occurred: coreViewModel.healthcheck.network_rollback_occurred,
				}
			: null

		// Boolean and string fields
		viewModel.is_authenticated = coreViewModel.is_authenticated
		viewModel.requires_password_set = coreViewModel.requires_password_set
		viewModel.is_loading = coreViewModel.is_loading
		viewModel.error_message = coreViewModel.error_message || null
		viewModel.success_message = coreViewModel.success_message || null
		viewModel.is_connected = coreViewModel.is_connected
		viewModel.auth_token = coreViewModel.auth_token || null

		// Sync the ref with the view model
		authToken.value = viewModel.auth_token

		// Device operation state - convert bincode variant to typed object
		viewModel.device_operation_state = convertDeviceOperationState(coreViewModel.device_operation_state)
		viewModel.reconnection_attempt = coreViewModel.reconnection_attempt
		viewModel.reconnection_timeout_seconds = coreViewModel.reconnection_timeout_seconds

		// Network change state
		viewModel.network_change_state = convertNetworkChangeState(coreViewModel.network_change_state)

		// Network form state
		viewModel.network_form_state = convertNetworkFormState(coreViewModel.network_form_state)

		// Network form dirty flag
		viewModel.network_form_dirty = coreViewModel.network_form_dirty

		// Browser hostname and current connection adapter (computed in Core)
		viewModel.browser_hostname = coreViewModel.browser_hostname || null
		viewModel.current_connection_adapter = coreViewModel.current_connection_adapter || null

		// Device offline tracking
		viewModel.device_went_offline = coreViewModel.device_went_offline

		// Network rollback modal state (computed in Core)
		viewModel.should_show_rollback_modal = coreViewModel.should_show_rollback_modal
		viewModel.default_rollback_enabled = coreViewModel.default_rollback_enabled

		// Firmware upload state
		viewModel.firmware_upload_state = convertUploadState(coreViewModel.firmware_upload_state)

		// Overlay spinner state
		// Preserve countdown_seconds if it's being actively managed by the Shell (network change polling)
		const isNetworkChangeActive = viewModel.network_change_state.type === 'waiting_for_new_ip'
		const preserveCountdown = isNetworkChangeActive && viewModel.overlay_spinner.countdown_seconds !== null

		viewModel.overlay_spinner = {
			overlay: coreViewModel.overlay_spinner.overlay,
			title: coreViewModel.overlay_spinner.title,
			text: coreViewModel.overlay_spinner.text || null,
			timed_out: coreViewModel.overlay_spinner.timed_out,
			progress: coreViewModel.overlay_spinner.progress !== null && coreViewModel.overlay_spinner.progress !== undefined
				? coreViewModel.overlay_spinner.progress
				: null,
			countdown_seconds: preserveCountdown
				? viewModel.overlay_spinner.countdown_seconds // Keep Shell's calculated value
				: (coreViewModel.overlay_spinner.countdown_seconds !== null && coreViewModel.overlay_spinner.countdown_seconds !== undefined
					? coreViewModel.overlay_spinner.countdown_seconds
					: null),
		}

		// Auto-subscribe logic based on authentication state transition
		if (viewModel.is_authenticated && !wasAuthenticated) {
			console.log('[useCore] User authenticated, triggering subscription')
			if (authToken.value && !isSubscribed.value && sendEventCallback) {
				isSubscribed.value = true
				sendEventCallback(new EventVariantWebSocket(new WebSocketEventVariantSubscribeToChannels()))
			}
		}

		// Reset subscription state on logout
		if (!viewModel.is_authenticated && wasAuthenticated) {
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
