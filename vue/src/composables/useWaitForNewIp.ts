import { ref } from "vue"
import { useEventHook } from "./useEventHook"

type FetchError = TypeError & {
	cause: {
		code: string
	}
}

const connectedEvent = useEventHook()

export function useWaitForNewIp() {
	const url = ref("")
	const reconnectInterval = ref()

	const startWaitForNewIp = async (newUrl: string) => {
		url.value = newUrl
		reconnectInterval.value = setInterval(checkForNewIp, 5_000)
	}

	const stopWaitForNewIp = () => {
		clearInterval(reconnectInterval.value)
		connectedEvent.trigger()
	}

	const checkForNewIp = async () => {
		try {
			const res = await fetch(`${url.value}/healthcheck`, {
				headers: {
					"Cache-Control": "no-cache, no-store, must-revalidate",
					Pragma: "no-cache",
					Expires: "0"
				}
			})

			if (res.status === 200) {
				stopWaitForNewIp()
			}
		} catch (error) {
			const e = error as FetchError

			if (e.name === "TypeError") {
				const code = e?.cause?.code
				if (code === "SELF_SIGNED_CERT_IN_CHAIN") {
					stopWaitForNewIp()
					return
				}
			}
		}
	}

	return { startWaitForNewIp, stopWaitForNewIp, onConnected: connectedEvent.on }
}
