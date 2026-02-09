import { Centrifuge, type PublicationContext, SubscriptionState } from "centrifuge"
import { type Ref, ref } from "vue"
import type { CentrifugeSubscriptionType } from "../enums/centrifuge-subscription-type.enum"
import { useEventHook } from "./useEventHook"

// Global state for the token ref, passed from useCore to avoid circular dependency
let globalAuthTokenRef: Ref<string | null> | undefined

const centrifuge: Ref<Centrifuge | undefined> = ref(undefined)
const connectedEvent = useEventHook()
const isConnected = ref(false)

export function useCentrifuge() {
	const centrifuge_url = `wss://${window.location.hostname}:8000/connection/websocket`

	// method to inject the auth token ref from useCore
	const setAuthToken = (tokenRef: Ref<string | null>) => {
		globalAuthTokenRef = tokenRef
	}

	const initializeCentrifuge = () => {
		if (!centrifuge.value) {
			if (!globalAuthTokenRef) {
				console.error("Centrifugo initialization error: authTokenRef not set. Call setAuthToken first.")
				return // Prevent crash
			}
			const token = globalAuthTokenRef?.value || undefined // Use undefined if empty to align with Centrifuge type

			centrifuge.value = new Centrifuge(centrifuge_url, {
				token: token,
				getToken: async () => {
					return await fetchAndRefreshCentrifugeToken()
				}
			})
			centrifuge.value
				.on("connecting", (ctx) => {
					console.debug(`connecting: ${ctx.code}, ${ctx.reason}`)
				})
				.on("connected", (ctx) => {
					isConnected.value = true
					connectedEvent.trigger()
					console.debug(`connected over ${ctx.transport}`)
				})
				.on("disconnected", (ctx) => {
					isConnected.value = false
					console.debug(`disconnected: ${ctx.code}, ${ctx.reason}`)
				})
				.connect()
		}
	}

	const disconnect = () => {
		if (centrifuge.value) {
			centrifuge.value.disconnect()
			centrifuge.value = undefined
			isConnected.value = false
		}
	}

	const fetchAndRefreshCentrifugeToken = async (): Promise<string> => {
		try {
			// Centrifuge's `getToken` callback expects a direct, synchronous-like
			// Promise resolution. To avoid complex async state management and event routing
			// through the Core for every token refresh, the Shell directly performs this
			// network request. It uses the `authToken` from useCore (which is populated
			// by the Core's LoginResponse) as a Bearer token.
			if (!globalAuthTokenRef) {
				console.error("fetchAndRefreshCentrifugeToken error: authTokenRef not set.")
				return ""
			}
			const token = globalAuthTokenRef.value
			const headers: HeadersInit = token
				? { Authorization: `Bearer ${token}` }
				: {}

			const res = await fetch("token/refresh", {
				credentials: "include",
				headers
			});

			if (res.ok) {
				return await res.text()
			}
			console.error(`Failed to refresh token: ${res.status} ${res.statusText}`)
		} catch (e) {
			console.error("Error refreshing token:", e)
		}

		return ""
	}

	const subscribe = async <T>(callback: (data: T) => void, channel: CentrifugeSubscriptionType) => {
		if (!centrifuge.value) {
			return undefined
		}
		let currentSub = centrifuge.value?.getSubscription(channel)

		if (currentSub === null) {
			currentSub = centrifuge.value.newSubscription(channel)
			currentSub
				.on("publication", (ctx: PublicationContext) => {
					console.debug(`publication ${ctx.channel}`, ctx.data)
					callback(ctx.data)
				})
				.on("subscribing", (ctx) => {
					console.debug(`subscribing: ${ctx.channel}, ${ctx.code}, ${ctx.reason}`)
				})
				.on("subscribed", (ctx) => {
					console.debug(`subscribed ${ctx.channel}`, ctx)
				})
				.on("unsubscribed", (ctx) => {
					console.debug(`unsubscribed: ${ctx.channel}, ${ctx.code}, ${ctx.reason}`)
				})
		}
		if (currentSub.state === SubscriptionState.Unsubscribed) {
			currentSub.subscribe()
		}
	}

	const history = async <T>(callback: (data: T) => void, channel: string) => {
		if (!centrifuge.value) {
			return
		}
		try {
			const res = await centrifuge.value.history(channel, { limit: 1 })
			const firstPub = res?.publications?.[0]
			if (firstPub?.data) {
				callback(firstPub.data as T)
			}
		} catch (e) {
			console.error(`[Centrifugo] History failed for ${channel}:`, e)
		}
	}

	const unsubscribe = (channel: string) => {
		const currentSub = centrifuge.value?.getSubscription(channel)
		if (currentSub != null && currentSub.state !== SubscriptionState.Unsubscribed) {
			currentSub.unsubscribe()
		}
	}

	const unsubscribeAll = () => {
		if (!centrifuge?.value?.subscriptions) return
		for (const subName of Object.keys(centrifuge.value.subscriptions())) {
			const sub = centrifuge.value.subscriptions()[subName]
			sub?.unsubscribe()
		}
	}

	return { subscribe, unsubscribe, unsubscribeAll, initializeCentrifuge, history, disconnect, onConnected: connectedEvent.on, isConnected, setAuthToken };
}
