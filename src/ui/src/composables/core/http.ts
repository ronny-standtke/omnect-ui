/**
 * HTTP capability implementation for Crux Core
 *
 * This module handles the Shell's implementation of the HTTP capability,
 * converting Core's HttpRequest into fetch() calls and returning responses.
 */

import { wasmModule } from './state'
import {
	HttpResponse as CoreHttpResponse,
	HttpHeader as CoreHttpHeader,
	HttpResultVariantOk,
	HttpResultVariantErr,
	HttpErrorVariantIo,
} from '../../../../shared_types/generated/typescript/types/shared_types'
import { BincodeSerializer } from '../../../../shared_types/generated/typescript/bincode/mod'

// Effects processor callback - set by effects.ts to avoid circular dependency
let processEffectsCallback: ((effectsBytes: Uint8Array) => Promise<void>) | null = null

/**
 * Set the effects processor callback (called from effects.ts)
 */
export function setEffectsProcessor(callback: (effectsBytes: Uint8Array) => Promise<void>): void {
	processEffectsCallback = callback
}

/**
 * Execute an HTTP request and return the result to the Core
 *
 * This is the shell's implementation of the HTTP capability.
 * It converts the Core's HttpRequest into a fetch() call,
 * then serializes the result back for the Core to process.
 */
export async function executeHttpRequest(
	requestId: number,
	httpRequest: { method: string; url: string; headers: Array<{ name: string; value: string }>; body: Uint8Array }
): Promise<void> {
	if (!wasmModule.value) {
		console.warn('WASM module not loaded, cannot execute HTTP request')
		return
	}

	try {
		const headers = new Headers()
		for (const header of httpRequest.headers) {
			headers.append(header.name, header.value)
		}

		const fetchOptions: RequestInit = {
			method: httpRequest.method,
			headers,
			credentials: 'include',
		}

		if (httpRequest.method !== 'GET' && httpRequest.method !== 'HEAD' && httpRequest.body.length > 0) {
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			fetchOptions.body = httpRequest.body as any
		}

		// URLs are prefixed with `https://relative` to satisfy validation.
		// This side strips the prefix to send a relative URL to avoid HTTPS certificate issues.
		let url = httpRequest.url
		if (url.startsWith('https://relative')) {
			url = url.replace('https://relative', '')
		}

		const response = await fetch(url, fetchOptions)

		// Convert response headers
		const responseHeaders: Array<CoreHttpHeader> = []
		const responseHeadersMap = new Headers(response.headers)
		responseHeadersMap.forEach((value, name) => {
			responseHeaders.push(new CoreHttpHeader(name, value))
		})

		// Get response body as bytes
		const bodyBuffer = await response.arrayBuffer()
		const bodyBytes = new Uint8Array(bodyBuffer)
		console.log(`[HTTP Effect ${requestId}] Response body: ${bodyBytes.length} bytes`)

		// Create HttpResponse
		const httpResponse = new CoreHttpResponse(response.status, responseHeaders, bodyBytes)

		// Create success result
		const result = new HttpResultVariantOk(httpResponse)

		const serializer = new BincodeSerializer()
		result.serialize(serializer)
		const resultBytes = serializer.getBytes()
		const newEffectsBytes = wasmModule.value.handle_response(requestId, resultBytes) as Uint8Array
		if (newEffectsBytes.length > 0 && processEffectsCallback) {
			await processEffectsCallback(newEffectsBytes)
		}
	} catch (error) {
		// Create error result
		const errorMessage = error instanceof Error ? error.message : String(error)
		console.error(`[HTTP Effect ${requestId}] Error:`, errorMessage)
		const httpError = new HttpErrorVariantIo(errorMessage)
		const result = new HttpResultVariantErr(httpError)

		const serializer = new BincodeSerializer()
		result.serialize(serializer)
		const resultBytes = serializer.getBytes()
		const newEffectsBytes = wasmModule.value.handle_response(requestId, resultBytes) as Uint8Array
		if (newEffectsBytes.length > 0 && processEffectsCallback) {
			await processEffectsCallback(newEffectsBytes)
		}
	}
}
