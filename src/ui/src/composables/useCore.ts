/**
 * Vue composable for integrating with Crux Core
 *
 * @see ./core/index.ts - Main composable and public API
 * @see ./core/types.ts - Type definitions and conversions
 * @see ./core/state.ts - Singleton reactive state
 * @see ./core/effects.ts - Effect processing
 * @see ./core/http.ts - HTTP capability
 * @see ./core/centrifugo.ts - WebSocket capability
 * @see ./core/timers.ts - Timer management
 * @see ./core/sync.ts - ViewModel synchronization
 */

// Re-export everything from core/index.ts
export { useCore } from './core'

// Re-export types
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
	DeviceNetwork,
} from './core'

// Re-export NetworkConfigRequest class
export { NetworkConfigRequest } from './core'
