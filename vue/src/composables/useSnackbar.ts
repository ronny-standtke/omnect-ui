import { createGlobalState } from "@vueuse/core"
import { reactive } from "vue"

export const useSnackbar = createGlobalState(() => {
	const snackbarState = reactive({
		color: "",
		timeout: -1,
		msg: "",
		snackbar: false
	})

	const reset = () => {
		snackbarState.color = ""
		snackbarState.timeout = -1
		snackbarState.msg = ""
		snackbarState.snackbar = false
	}

	const showError = (msg: string) => {
		snackbarState.color = "error"
		snackbarState.timeout = -1
		snackbarState.msg = msg
		snackbarState.snackbar = true
	}

	return { snackbarState, reset, showError }
})
