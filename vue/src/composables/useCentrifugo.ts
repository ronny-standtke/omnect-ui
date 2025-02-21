import { Centrifuge, type PublicationContext, SubscriptionState } from "centrifuge"
import { type Ref, ref } from "vue"
import type { CentrifugeSubscriptionType } from "../enums/centrifuge-subscription-type.enum"
import { useEventHook } from "./useEventHook"

const centrifuge: Ref<Centrifuge | undefined> = ref(undefined)
const connectedEvent = useEventHook()

export function useCentrifuge() {
	const centrifuge_url = `wss://${window.location.hostname}:8000/connection/websocket`
	const token = ref("")

	const initializeCentrifuge = () => {
		if (centrifuge.value == null) {
			centrifuge.value = new Centrifuge(centrifuge_url, {
				token: token.value,
				getToken: async () => {
					return await getToken()
				}
			})
			centrifuge.value
				.on("connecting", (ctx) => {
					console.log(`connecting: ${ctx.code}, ${ctx.reason}`)
				})
				.on("connected", (ctx) => {
					console.log(`connected over ${ctx.transport}`)
					connectedEvent.trigger()
				})
				.on("disconnected", (ctx) => {
					console.log(`disconnected: ${ctx.code}, ${ctx.reason}`)
				})
				.connect()
		}
	}

	const getToken = async (): Promise<string> => {
		const res = await fetch("token/refresh")

		if (res.ok) {
			return await res.text()
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
					console.log(`publication ${ctx.channel}`, ctx.data)
					callback(ctx.data)
				})
				.on("subscribing", (ctx) => {
					console.log(`subscribing: ${ctx.channel}, ${ctx.code}, ${ctx.reason}`)
				})
				.on("subscribed", (ctx) => {
					console.log(`subscribed ${ctx.channel}`, ctx)
				})
				.on("unsubscribed", (ctx) => {
					console.log(`unsubscribed: ${ctx.channel}, ${ctx.code}, ${ctx.reason}`)
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
		const res = await centrifuge.value.history(channel, { limit: 1 })
		if (res?.publications.length > 0) {
			callback(res.publications[0].data as T)
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

	return { subscribe, unsubscribe, unsubscribeAll, initializeCentrifuge, token, history, onConnected: connectedEvent.on }
}
