import axios, { AxiosError } from "axios"
import { ref } from "vue"
import { useEventHook } from "./useEventHook"

const connectedEvent = useEventHook()

export function useWaitReconnect() {
	const wasDown = ref(false)
	const reconnectInterval = ref()

	const startWaitReconnect = async () => {
		reconnectInterval.value = setInterval(checkReconnect, 5_000)
	}

	const stopWaitReconnect = () => {
		wasDown.value = false
		clearInterval(reconnectInterval.value)
		connectedEvent.trigger()
	}

	const checkReconnect = async () => {
		try {
			const res = await axios.get("/healthcheck", {
				headers: {
					"Cache-Control": "no-cache, no-store, must-revalidate",
					Pragma: "no-cache",
					Expires: "0"
				},
				timeout: 2_500
			})
			if (res.status === 200 && wasDown.value) {
				stopWaitReconnect()
			}
		} catch (error) {
			const err = error as AxiosError
			if (err.isAxiosError && err.code === "ECONNABORTED") {
				wasDown.value = true
			}
		}
	}

	return { startWaitReconnect, stopWaitReconnect, onConnected: connectedEvent.on }
}
