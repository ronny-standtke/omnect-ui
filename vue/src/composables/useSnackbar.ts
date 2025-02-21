import { createGlobalState } from "@vueuse/core"
import { reactive } from "vue"

export const useSnackbar = createGlobalState(() => {
	const snackbarState = reactive({
		color: "",
		timeout: -1,
		msg: "",
		snackbar: false
	})

	return { snackbarState }
})
