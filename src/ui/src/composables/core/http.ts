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
	if (!wasmModule) {
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

		// Workaround: `crux_http` in the Rust core panics on relative URLs.
		// The Rust side prefixes URLs with `http://omnect-device` to satisfy `crux_http`'s validation.
		// This side strips the prefix to send a relative URL, which `fetch` handles correctly.
		let url = httpRequest.url
		if (url.startsWith('http://omnect-device')) {
			url = url.replace('http://omnect-device', '')
		}

		const response = await fetch(url, fetchOptions)

		// Workaround: crux_http (0.15) appears to discard the response body for 4xx/5xx errors
		// and returns a generic error. To preserve the body (which contains validation messages),
		// we map error statuses to 200 OK and pass the original status in a header.
		// The Core macro will detect this header and treat it as an error.
		let status = response.status
		const responseHeadersMap = new Headers(response.headers)
		if (status >= 400) {
			console.log(`[HTTP Effect ${requestId}] Masking status ${status} as 200 to preserve body`)
			responseHeadersMap.append('x-original-status', status.toString())
			status = 200
		}

		// Convert response headers
		const responseHeaders: Array<CoreHttpHeader> = []
		responseHeadersMap.forEach((value, name) => {
			responseHeaders.push(new CoreHttpHeader(name, value))
		})

		// Get response body as bytes
		const bodyBuffer = await response.arrayBuffer()
		const bodyBytes = new Uint8Array(bodyBuffer)
		console.log(`[HTTP Effect ${requestId}] Response body: ${bodyBytes.length} bytes`)

		// Create HttpResponse
		const httpResponse = new CoreHttpResponse(status, responseHeaders, bodyBytes)

		// Create success result
		const result = new HttpResultVariantOk(httpResponse)

		const serializer = new BincodeSerializer()
		result.serialize(serializer)
		const resultBytes = serializer.getBytes()
		const newEffectsBytes = wasmModule.handle_response(requestId, resultBytes) as Uint8Array
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
		const newEffectsBytes = wasmModule.handle_response(requestId, resultBytes) as Uint8Array
		if (newEffectsBytes.length > 0 && processEffectsCallback) {
			await processEffectsCallback(newEffectsBytes)
		}
	}
}
