/**
 * Effect processing for Crux Core
 *
 * This module processes effects returned by the Crux Core:
 * - Render: Update the viewModel from Core
 * - Http: Execute HTTP requests
 * - Centrifugo: Handle WebSocket subscriptions
 */

import { wasmModule } from './state'
import { executeHttpRequest, setEffectsProcessor as setHttpEffectsProcessor } from './http'
import { executeCentrifugoOperation, setEffectsProcessor as setCentrifugoEffectsProcessor } from './centrifugo'
import {
	Request as CruxRequest,
	EffectVariantRender,
	EffectVariantHttp,
	EffectVariantCentrifugo,
} from '../../../../shared_types/generated/typescript/types/shared_types'
import { BincodeDeserializer } from '../../../../shared_types/generated/typescript/bincode/mod'

// ViewModel updater callback - set by sync.ts to avoid circular dependency
let updateViewModelCallback: (() => void) | null = null

/**
 * Set the ViewModel updater callback (called from sync.ts)
 */
export function setViewModelUpdater(callback: () => void): void {
	updateViewModelCallback = callback
}

/**
 * Process effects from the Crux Core
 *
 * Effects are the Core's way of requesting the Shell to perform side effects:
 * - Render: Fetch and update the viewModel from Core
 * - Http: Execute HTTP requests and send responses back to Core
 * - Centrifugo: Subscribe to WebSocket channels (only SubscribeAll)
 */
export async function processEffects(effectsBytes: Uint8Array): Promise<void> {
	if (!wasmModule) {
		console.warn('WASM module not loaded, cannot process effects')
		return
	}

	// Deserialize effects from bincode (array of Request objects)
	const deserializer = new BincodeDeserializer(effectsBytes)
	const numRequests = deserializer.deserializeLen()

	for (let i = 0; i < numRequests; i++) {
		const request = CruxRequest.deserialize(deserializer)
		const effect = request.effect

		if (effect instanceof EffectVariantRender) {
			// Render effect: Update the view model from core
			if (updateViewModelCallback) {
				updateViewModelCallback()
			}
		} else if (effect instanceof EffectVariantHttp) {
			// HTTP effect: Execute HTTP request and send response back to core
			const httpRequest = effect.value
			console.log(`HTTP ${httpRequest.method} ${httpRequest.url}`)

			// Execute the request asynchronously (don't await to allow parallel processing)
			executeHttpRequest(request.id, {
				method: httpRequest.method,
				url: httpRequest.url,
				headers: httpRequest.headers.map((h) => ({ name: h.name, value: h.value })),
				body: httpRequest.body,
			}).catch((error) => {
				console.error('Failed to execute HTTP request:', error)
			})
		} else if (effect instanceof EffectVariantCentrifugo) {
			// Centrifugo effect: Handle WebSocket subscription
			const centrifugoOperation = effect.value
			console.log(`Centrifugo operation:`, centrifugoOperation)

			// Execute the operation asynchronously
			executeCentrifugoOperation(request.id, centrifugoOperation).catch((error) => {
				console.error('Failed to execute Centrifugo operation:', error)
			})
		} else {
			console.warn('Unknown effect type:', effect)
		}
	}
}

// Wire up the circular dependency: http.ts and centrifugo.ts need to call processEffects
setHttpEffectsProcessor(processEffects)
setCentrifugoEffectsProcessor(processEffects)
