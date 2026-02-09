/**
 * Centrifugo capability implementation for Crux Core
 *
 * This module handles WebSocket subscriptions and message parsing
 * for real-time updates from omnect-device-service (ODS).
 */

import { centrifugoInstance, wasmModule } from './state'
import { CentrifugeSubscriptionType } from '../../enums/centrifuge-subscription-type.enum'
import {
	EventVariantWebSocket,
	WebSocketEventVariantOnlineStatusUpdated,
	WebSocketEventVariantSystemInfoUpdated,
	WebSocketEventVariantNetworkStatusUpdated,
	WebSocketEventVariantFactoryResetUpdated,
	WebSocketEventVariantUpdateValidationStatusUpdated,
	WebSocketEventVariantTimeoutsUpdated,
	CentrifugoOperationVariantSubscribeAll,
	CentrifugoOperationVariantUnsubscribeAll,
	CentrifugoOutputVariantConnected,
	CentrifugoOutputVariantDisconnected,
	CentrifugoOutputVariantError,
	type Event,
} from '../../../../shared_types/generated/typescript/types/shared_types'
import { BincodeSerializer } from '../../../../shared_types/generated/typescript/bincode/mod'

// Event sender callback - set by index.ts to avoid circular dependency
let sendEventCallback: ((event: Event) => Promise<void>) | null = null

// Effects processor callback - set by effects.ts to avoid circular dependency
let processEffectsCallback: ((effectsBytes: Uint8Array) => Promise<void>) | null = null

/**
 * Set the event sender callback (called from index.ts after initialization)
 */
export function setEventSender(callback: (event: Event) => Promise<void>): void {
	sendEventCallback = callback
}

/**
 * Set the effects processor callback (called from effects.ts)
 */
export function setEffectsProcessor(callback: (effectsBytes: Uint8Array) => Promise<void>): void {
	processEffectsCallback = callback
}

/**
 * Parse WebSocket channel data from ODS JSON and send as typed event to Core
 *
 * Architecture:
 * - Receives JSON from Centrifugo WebSocket (ODS data format)
 * - Sends raw JSON string to Core
 * - Core parses JSON and constructs internal types
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
				await sendEventCallback(new EventVariantWebSocket(new WebSocketEventVariantOnlineStatusUpdated(jsonData)))
				break
			}
			case 'SystemInfoV1': {
				await sendEventCallback(new EventVariantWebSocket(new WebSocketEventVariantSystemInfoUpdated(jsonData)))
				break
			}
			case 'TimeoutsV1': {
				await sendEventCallback(new EventVariantWebSocket(new WebSocketEventVariantTimeoutsUpdated(jsonData)))
				break
			}
			case 'NetworkStatusV1': {
				await sendEventCallback(new EventVariantWebSocket(new WebSocketEventVariantNetworkStatusUpdated(jsonData)))
				break
			}
			case 'FactoryResetV1': {
				await sendEventCallback(new EventVariantWebSocket(new WebSocketEventVariantFactoryResetUpdated(jsonData)))
				break
			}
			case 'UpdateValidationStatusV1': {
				await sendEventCallback(
					new EventVariantWebSocket(new WebSocketEventVariantUpdateValidationStatusUpdated(jsonData))
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
 * Execute Centrifugo operation
 *
 * Subscribes to all Centrifugo channels and forwards messages as events to Core.
 * Uses the event-based architecture where WebSocket data is parsed and sent as
 * typed events (*Updated) rather than responses.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function executeCentrifugoOperation(requestId: number, operation: any): Promise<void> {
	const sendResponse = async (output: any) => {
		if (!wasmModule.value) return
		const serializer = new BincodeSerializer()
		output.serialize(serializer)
		const responseBytes = serializer.getBytes()
		const newEffectsBytes = wasmModule.value.handle_response(requestId, responseBytes) as Uint8Array
		if (newEffectsBytes.length > 0 && processEffectsCallback) {
			await processEffectsCallback(newEffectsBytes)
		}
	}

	if (operation instanceof CentrifugoOperationVariantSubscribeAll) {
		const channels = Object.values(CentrifugeSubscriptionType)
		centrifugoInstance.initializeCentrifuge()

		let subscriptionsStarted = false
		const performSubscriptions = async () => {
			if (subscriptionsStarted) return
			subscriptionsStarted = true

			try {
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
				await sendResponse(new CentrifugoOutputVariantConnected())
			} catch (error) {
				const errorMessage = error instanceof Error ? error.message : String(error)
				await sendResponse(new CentrifugoOutputVariantError(errorMessage))
			}
		}

		centrifugoInstance.onConnected(() => {
			performSubscriptions()
		})
		performSubscriptions()
	} else if (operation instanceof CentrifugoOperationVariantUnsubscribeAll) {
		centrifugoInstance.disconnect()
		await sendResponse(new CentrifugoOutputVariantDisconnected())
	} else {
		console.error(`[Centrifugo] Unsupported operation`)
		await sendResponse(new CentrifugoOutputVariantError('Unsupported operation'))
	}
}