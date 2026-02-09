/**
 * Type definitions and conversions for Crux Core integration
 *
 * This module provides:
 * - Re-exports of generated types from shared_types
 * - Type conversion helpers for DeviceOperationState, NetworkChangeState, etc.
 * - ODS JSON to Crux type mappings
 */

// Re-export generated types from shared_types for use in Vue components
export type {
	SystemInfo,
	NetworkStatus,
	OnlineStatus,
	FactoryReset,
	UpdateValidationStatus,
	Timeouts,
	HealthcheckInfo,
	Event,
	Effect,
	Model as CoreViewModel,
	UpdateManifest,
	NetworkFormData,
} from '../../../../shared_types/generated/typescript/types/shared_types'

export { NetworkConfigRequest } from '../../../../shared_types/generated/typescript/types/shared_types'

// Import types and variant classes for conversions
import {
	DeviceOperationState,
	DeviceOperationStateVariantidle,
	DeviceOperationStateVariantrebooting,
	DeviceOperationStateVariantfactoryResetting,
	DeviceOperationStateVariantupdating,
	DeviceOperationStateVariantwaitingReconnection,
	DeviceOperationStateVariantreconnectionFailed,
	DeviceOperationStateVariantreconnectionSuccessful,
	UpdateManifest,
	NetworkChangeState,
	NetworkChangeStateVariantidle,
	NetworkChangeStateVariantapplyingConfig,
	NetworkChangeStateVariantwaitingForNewIp,
	NetworkChangeStateVariantnewIpReachable,
	NetworkChangeStateVariantnewIpTimeout,
	NetworkChangeStateVariantwaitingForOldIp,
	NetworkFormState,
	NetworkFormStateVariantidle,
	NetworkFormStateVariantediting,
	NetworkFormStateVariantsubmitting,
	FactoryResetStatus,
	FactoryResetStatusVariantunknown,
	FactoryResetStatusVariantmodeSupported,
	FactoryResetStatusVariantmodeUnsupported,
	FactoryResetStatusVariantbackupRestoreError,
	FactoryResetStatusVariantconfigurationError,
	UploadState,
	UploadStateVariantidle,
	UploadStateVariantuploading,
	UploadStateVariantcompleted,
	UploadStateVariantfailed,
	DeviceNetwork,
} from '../../../../shared_types/generated/typescript/types/shared_types'

// Re-export variant classes for external use
export {
	DeviceOperationState,
	NetworkChangeState,
	NetworkFormState,
	FactoryResetStatus,
	UploadState,
	DeviceNetwork,
}

// ============================================================================
// TypeScript Discriminated Union Types
// ============================================================================

export type DeviceOperationStateType =
	| { type: 'idle' }
	| { type: 'rebooting' }
	| { type: 'factoryResetting' }
	| { type: 'updating' }
	| { type: 'waitingReconnection'; operation: string; attempt: number }
	| { type: 'reconnectionFailed'; operation: string; reason: string }
	| { type: 'reconnectionSuccessful'; operation: string }

export type NetworkChangeStateType =
	| { type: 'idle' }
	| { type: 'applyingConfig'; isServerAddr: boolean; ipChanged: boolean; newIp: string; oldIp: string; switchingToDhcp: boolean }
	| { type: 'waitingForNewIp'; newIp: string; oldIp: string; attempt: number; uiPort: number; rollbackTimeoutSeconds: number; switchingToDhcp: boolean }
	| { type: 'newIpReachable'; newIp: string; uiPort: number }
	| { type: 'newIpTimeout'; newIp: string; oldIp: string; uiPort: number; switchingToDhcp: boolean }
	| { type: 'waitingForOldIp'; oldIp: string; uiPort: number; attempt: number }

export type NetworkFormStateType =
	| { type: 'idle' }
	| { type: 'editing'; adapterName: string; formData: NetworkFormDataType; errors: Record<string, string> }
	| { type: 'submitting'; adapterName: string; formData: NetworkFormDataType; errors: Record<string, string> }

export type UploadStateType =
	| { type: 'idle' }
	| { type: 'uploading' }
	| { type: 'completed' }
	| { type: 'failed'; content: string }

export interface NetworkFormDataType {
	name: string
	ipAddress: string
	dhcp: boolean
	subnetMask: string
	dns: string[]
	gateways: string[]
}

export interface OverlaySpinnerStateType {
	overlay: boolean
	title: string
	text: string | null
	timedOut: boolean
	progress: number | null
	countdownSeconds: number | null
}

export type FactoryResetStatusString = 'unknown' | 'modeSupported' | 'modeUnsupported' | 'backupRestoreError' | 'configurationError'

// ============================================================================
// ViewModel Interface
// ============================================================================

export interface ViewModel {
	systemInfo: {
		os: { name: string; version: string }
		azureSdkVersion: string
		omnectDeviceServiceVersion: string
		bootTime: string | null
	} | null
	networkStatus: {
		networkStatus: DeviceNetwork[]
	} | null
	onlineStatus: { iothub: boolean } | null
	factoryReset: {
		keys: string[]
		result: {
			status: FactoryResetStatusString
			context: string | null
			error: string
			paths: string[]
		} | null
	} | null
	updateValidationStatus: { status: string } | null
	updateManifest: UpdateManifest | null
	timeouts: { waitOnlineTimeout: { nanos: number; secs: bigint } } | null
	healthcheck: {
		versionInfo: { required: string; current: string; mismatch: boolean }
		updateValidationStatus: { status: string }
		networkRollbackOccurred: boolean
	} | null
	isAuthenticated: boolean
	requiresPasswordSet: boolean
	isLoading: boolean
	errorMessage: string | null
	successMessage: string | null
	isConnected: boolean
	authToken: string | null

	// Device operation state (reboot/factory reset reconnection)
	deviceOperationState: DeviceOperationStateType
	reconnectionAttempt: number

	// Network change state (IP change detection and polling)
	networkChangeState: NetworkChangeStateType

	// Network form state
	networkFormState: NetworkFormStateType

	// Network form dirty flag (tracks unsaved changes)
	networkFormDirty: boolean

	// Browser hostname
	browserHostname: string | null

	// Current connection adapter name
	currentConnectionAdapter: string | null

	// Device offline tracking
	deviceWentOffline: boolean

	// Network rollback modal state
	shouldShowRollbackModal: boolean
	defaultRollbackEnabled: boolean

	// Firmware upload state
	firmwareUploadState: UploadStateType

	// Overlay spinner state
	overlaySpinner: OverlaySpinnerStateType
}

// ============================================================================
// Type Conversion Functions
// ============================================================================

/**
 * Convert FactoryResetStatus class variant to string literal
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function factoryResetStatusToString(status: any): FactoryResetStatusString {
	if (status instanceof FactoryResetStatusVariantunknown) return 'unknown'
	if (status instanceof FactoryResetStatusVariantmodeSupported) return 'modeSupported'
	if (status instanceof FactoryResetStatusVariantmodeUnsupported) return 'modeUnsupported'
	if (status instanceof FactoryResetStatusVariantbackupRestoreError) return 'backupRestoreError'
	if (status instanceof FactoryResetStatusVariantconfigurationError) return 'configurationError'
	return 'unknown'
}

/**
 * Convert DeviceOperationState variant to typed object
 */
export function convertDeviceOperationState(state: DeviceOperationState): DeviceOperationStateType {
	if (state instanceof DeviceOperationStateVariantidle) {
		return { type: 'idle' }
	}
	if (state instanceof DeviceOperationStateVariantrebooting) {
		return { type: 'rebooting' }
	}
	if (state instanceof DeviceOperationStateVariantfactoryResetting) {
		return { type: 'factoryResetting' }
	}
	if (state instanceof DeviceOperationStateVariantupdating) {
		return { type: 'updating' }
	}
	if (state instanceof DeviceOperationStateVariantwaitingReconnection) {
		return { type: 'waitingReconnection', operation: state.operation, attempt: state.attempt }
	}
	if (state instanceof DeviceOperationStateVariantreconnectionFailed) {
		return { type: 'reconnectionFailed', operation: state.operation, reason: state.reason }
	}
	if (state instanceof DeviceOperationStateVariantreconnectionSuccessful) {
		return { type: 'reconnectionSuccessful', operation: state.operation }
	}
	return { type: 'idle' }
}

/**
 * Convert NetworkChangeState variant to typed object
 */
export function convertNetworkChangeState(state: NetworkChangeState): NetworkChangeStateType {
	if (state instanceof NetworkChangeStateVariantidle) {
		return { type: 'idle' }
	}
	if (state instanceof NetworkChangeStateVariantapplyingConfig) {
		return {
			type: 'applyingConfig',
			isServerAddr: state.is_server_addr,
			ipChanged: state.ip_changed,
			newIp: state.new_ip,
			oldIp: state.old_ip,
			switchingToDhcp: state.switching_to_dhcp,
		}
	}
	if (state instanceof NetworkChangeStateVariantwaitingForNewIp) {
		return {
			type: 'waitingForNewIp',
			newIp: state.new_ip,
			oldIp: state.old_ip,
			attempt: state.attempt,
			uiPort: state.ui_port,
			rollbackTimeoutSeconds: Number(state.rollback_timeout_seconds),
			switchingToDhcp: state.switching_to_dhcp,
		}
	}
	if (state instanceof NetworkChangeStateVariantnewIpReachable) {
		return { type: 'newIpReachable', newIp: state.new_ip, uiPort: state.ui_port }
	}
	if (state instanceof NetworkChangeStateVariantnewIpTimeout) {
		return {
			type: 'newIpTimeout',
			newIp: state.new_ip,
			oldIp: state.old_ip,
			uiPort: state.ui_port,
			switchingToDhcp: state.switching_to_dhcp,
		}
	}
	if (state instanceof NetworkChangeStateVariantwaitingForOldIp) {
		return {
			type: 'waitingForOldIp',
			oldIp: state.old_ip,
			uiPort: state.ui_port,
			attempt: state.attempt,
		}
	}
	return { type: 'idle' }
}

/**
 * Convert NetworkFormState variant to typed object
 */
export function convertNetworkFormState(state: NetworkFormState): NetworkFormStateType {
	if (state instanceof NetworkFormStateVariantidle) {
		return { type: 'idle' }
	}
	if (state instanceof NetworkFormStateVariantediting) {
		return {
			type: 'editing',
			adapterName: state.adapter_name,
			formData: {
				name: state.form_data.name,
				ipAddress: state.form_data.ipAddress,
				dhcp: state.form_data.dhcp,
				subnetMask: state.form_data.subnetMask,
				dns: [...state.form_data.dns],
				gateways: [...state.form_data.gateways],
			},
			errors: state.errors instanceof Map ? Object.fromEntries(state.errors) : state.errors,
		}
	}
	if (state instanceof NetworkFormStateVariantsubmitting) {
		return {
			type: 'submitting',
			adapterName: state.adapter_name,
			formData: {
				name: state.form_data.name,
				ipAddress: state.form_data.ipAddress,
				dhcp: state.form_data.dhcp,
				subnetMask: state.form_data.subnetMask,
				dns: [...state.form_data.dns],
				gateways: [...state.form_data.gateways],
			},
			errors: state.errors instanceof Map ? Object.fromEntries(state.errors) : state.errors,
		}
	}
	return { type: 'idle' }
}

/**
 * Convert UploadState variant to typed object
 */
export function convertUploadState(state: UploadState): UploadStateType {
	if (state instanceof UploadStateVariantidle) {
		return { type: 'idle' }
	}
	if (state instanceof UploadStateVariantuploading) {
		return { type: 'uploading' }
	}
	if (state instanceof UploadStateVariantcompleted) {
		return { type: 'completed' }
	}
	if (state instanceof UploadStateVariantfailed) {
		return { type: 'failed', content: state.value }
	}
	return { type: 'idle' }
}
