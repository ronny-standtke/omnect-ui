import axios, { AxiosError } from "axios"
import { ref } from "vue"
import type { HealthcheckResponse } from "../types"
import { useEventHook } from "./useEventHook"

const connectedEvent = useEventHook()
const timeoutEvent = useEventHook()

export function useAwaitUpdate() {
	const wasDown = ref(false)
	const interval = ref()
	const timeout = ref()

	const startWaitReconnect = async () => {
		interval.value = setInterval(checkUpdateState, 5_000)
		timeout.value = setTimeout(() => {
			timeoutEvent.trigger()
			clearTimeout(timeout.value)
		}, 300_000)
	}

	const stopWaitReconnect = () => {
		wasDown.value = false
		clearInterval(interval.value)
		clearTimeout(timeout.value)
		connectedEvent.trigger()
	}

	const checkUpdateState = async () => {
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
				const data = res.data as HealthcheckResponse
				if (data.update_validation_status.status === "Succeeded" || data.update_validation_status.status === "Recovered") {
					stopWaitReconnect()
				}
			}
		} catch (error) {
			const err = error as AxiosError
			if (err.isAxiosError && err.code === "ECONNABORTED") {
				wasDown.value = true
			}
		}
	}

	return { startWaitReconnect, stopWaitReconnect, onUpdateDone: connectedEvent.on, onTimeout: timeoutEvent.on }
}
