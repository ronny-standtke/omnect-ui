<script setup lang="ts">
import { useFetch } from "@vueuse/core"
import { computed } from "vue"
import { useRouter } from "vue-router"
import { useSnackbar } from "../composables/useSnackbar"

const { showError, snackbarState } = useSnackbar()
const router = useRouter()

const loading = computed(() => reloadNetworkLoading.value)

const {
	onFetchError: onReloadNetworkError,
	error: reloadNetworkError,
	statusCode: reloadNetworkStatusCode,
	onFetchResponse: onReloadNetworkSuccess,
	execute: reloadNetwork,
	isFetching: reloadNetworkLoading
} = useFetch("reload-network", { immediate: false }).post()

onReloadNetworkSuccess(() => {
	showSuccess("Restart network successful.")
})

onReloadNetworkError(() => {
	if (reloadNetworkStatusCode.value === 401) {
		router.push("/login")
	} else {
		showError(`Reloading network failed: ${JSON.stringify(reloadNetworkError.value)}`)
	}
})

const showSuccess = (successMsg: string) => {
	snackbarState.msg = successMsg
	snackbarState.color = "success"
	snackbarState.timeout = 2000
	snackbarState.snackbar = true
}
</script>

<template>
    <div class="flex flex-col gap-y-4 items-start">
        <div class="text-h4 text-secondary border-b w-100">Commands</div>
        <v-btn :prepend-icon="'mdi-refresh'" variant="text" :loading="reloadNetworkLoading" :disabled="loading"
            @click="reloadNetwork(false)">
            Restart network
        </v-btn>
    </div>
</template>