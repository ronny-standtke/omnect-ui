import { createGlobalState } from "@vueuse/core"
import { reactive } from "vue"
import { useEventHook } from "./useEventHook"

export const useOverlaySpinner = createGlobalState(() => {
	const updateDone = useEventHook()

	const overlaySpinnerState = reactive({
		overlay: false,
		title: "",
		text: "",
		isUpdateRunning: false
	})

	const reset = () => {
		overlaySpinnerState.overlay = false
		overlaySpinnerState.text = ""
		overlaySpinnerState.title = ""
		overlaySpinnerState.isUpdateRunning = false
	}

	return { overlaySpinnerState, reset, updateDone }
})
