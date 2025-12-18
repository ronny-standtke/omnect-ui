/**
 * Centrifugo capability implementation for Crux Core
 *
 * This module handles WebSocket subscriptions and message parsing
 * for real-time updates from omnect-device-service (ODS).
 */

import { centrifugoInstance } from './state'
import { stringToFactoryResetStatus } from './types'
import { CentrifugeSubscriptionType } from '../../enums/centrifuge-subscription-type.enum'
import type {
	OdsOnlineStatus,
	OdsSystemInfo,
	OdsTimeouts,
	OdsNetworkStatus,
	OdsFactoryReset,
	OdsUpdateValidationStatus,
} from '../../types/ods'
import {
	EventVariantWebSocket,
	WebSocketEventVariantOnlineStatusUpdated,
	WebSocketEventVariantSystemInfoUpdated,
	WebSocketEventVariantNetworkStatusUpdated,
	WebSocketEventVariantFactoryResetUpdated,
	WebSocketEventVariantUpdateValidationStatusUpdated,
	WebSocketEventVariantTimeoutsUpdated,
	OnlineStatus,
	SystemInfo,
	OsInfo,
	NetworkStatus,
	DeviceNetwork,
	InternetProtocol,
	IpAddress,
	FactoryReset,
	FactoryResetResult,
	UpdateValidationStatus,
	Timeouts,
	Duration,
	CentrifugoOperationVariantSubscribeAll,
	type Event,
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
 * Parse WebSocket channel data from ODS JSON and send as typed event to Core
 *
 * Architecture:
 * - Receives JSON from Centrifugo WebSocket (ODS data format)
 * - Parses JSON and constructs typed TypeScript class instances
 * - Sends as *Updated events to Core (not responses)
 * - Core processes events, updates Model, and renders
 * - Shell reads updated viewModel from Core
 *
 * This event-based approach avoids request/response conflicts with streaming data.
 */
async function parseAndSendChannelEvent(channel: string, jsonData: string): Promise<void> {
	if (!sendEventCallback) {
		console.warn('[Centrifugo] Event sender not initialized')
		return
	}

	try {
		switch (channel) {
			case 'OnlineStatusV1': {
				const json = JSON.parse(jsonData) as OdsOnlineStatus
				const data = new OnlineStatus(json.iothub)
				await sendEventCallback(new EventVariantWebSocket(new WebSocketEventVariantOnlineStatusUpdated(data)))
				break
			}
			case 'SystemInfoV1': {
				const json = JSON.parse(jsonData) as OdsSystemInfo
				const data = new SystemInfo(
					new OsInfo(json.os?.name || '', json.os?.version || ''),
					json.azure_sdk_version || '',
					json.omnect_device_service_version || '',
					json.boot_time ? String(json.boot_time) : null
				)
				await sendEventCallback(new EventVariantWebSocket(new WebSocketEventVariantSystemInfoUpdated(data)))
				break
			}
			case 'TimeoutsV1': {
				const json = JSON.parse(jsonData) as OdsTimeouts
				const data = new Timeouts(
					new Duration(json.wait_online_timeout?.nanos || 0, BigInt(json.wait_online_timeout?.secs || 0))
				)
				await sendEventCallback(new EventVariantWebSocket(new WebSocketEventVariantTimeoutsUpdated(data)))
				break
			}
			case 'NetworkStatusV1': {
				const json = JSON.parse(jsonData) as OdsNetworkStatus
				console.log('NetworkStatusV1 WebSocket update received:', json)
				const networks = (json.network_status || []).map((net) => {
					console.log('Network adapter:', net.name, 'dhcp:', net.ipv4?.addrs[0]?.dhcp)
					return new DeviceNetwork(
						new InternetProtocol(
							(net.ipv4?.addrs || []).map(
								(addr) => new IpAddress(addr.addr || '', addr.dhcp || false, addr.prefix_len || 0)
							),
							net.ipv4?.dns || [],
							net.ipv4?.gateways || []
						),
						net.mac || '',
						net.name || '',
						net.online || false,
						net.file || null
					)
				})
				const data = new NetworkStatus(networks)
				await sendEventCallback(new EventVariantWebSocket(new WebSocketEventVariantNetworkStatusUpdated(data)))
				break
			}
			case 'FactoryResetV1': {
				const json = JSON.parse(jsonData) as OdsFactoryReset
				const result = json.result
					? new FactoryResetResult(
							stringToFactoryResetStatus(json.result.status || 'unknown'),
							json.result.context || null,
							json.result.error || '',
							json.result.paths || []
						)
					: null
				const data = new FactoryReset(json.keys || [], result)
				await sendEventCallback(new EventVariantWebSocket(new WebSocketEventVariantFactoryResetUpdated(data)))
				break
			}
			case 'UpdateValidationStatusV1': {
				const json = JSON.parse(jsonData) as OdsUpdateValidationStatus
				const data = new UpdateValidationStatus(json.status || '')
				await sendEventCallback(
					new EventVariantWebSocket(new WebSocketEventVariantUpdateValidationStatusUpdated(data))
				)
				break
			}
			default:
				console.warn(`[Centrifugo] Unknown channel: ${channel}`)
		}
	} catch (error) {
		console.error(`[Centrifugo] Error parsing ${channel}:`, error)
	}
}

/**
 * Execute Centrifugo SubscribeAll operation
 *
 * Subscribes to all Centrifugo channels and forwards messages as events to Core.
 * Uses the event-based architecture where WebSocket data is parsed and sent as
 * typed events (*Updated) rather than responses.
 *
 * Note: Only SubscribeAll is implemented - individual channel operations removed.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function executeCentrifugoOperation(operation: any): Promise<void> {
	if (operation instanceof CentrifugoOperationVariantSubscribeAll) {
		const channels = Object.values(CentrifugeSubscriptionType)
		centrifugoInstance.initializeCentrifuge()

		let subscriptionsStarted = false
		const performSubscriptions = async () => {
			if (subscriptionsStarted) return
			subscriptionsStarted = true

			for (const channel of channels) {
				await centrifugoInstance.subscribe((data: unknown) => {
					const jsonData = JSON.stringify(data)
					parseAndSendChannelEvent(channel, jsonData)
				}, channel)

				await centrifugoInstance.history((data: unknown) => {
					try {
						if (data) {
							const jsonData = JSON.stringify(data)
							parseAndSendChannelEvent(channel, jsonData)
						}
					} catch (error) {
						console.error(`[Centrifugo] Error processing history for ${channel}:`, error)
					}
				}, channel)
			}
		}

		centrifugoInstance.onConnected(() => {
			performSubscriptions()
		})
		performSubscriptions()
	} else {
		console.error(`[Centrifugo] Unsupported operation - only SubscribeAll is implemented`)
	}
}
