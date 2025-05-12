<script setup lang="ts">
import { useFetch } from "@vueuse/core"
import { type Ref, computed, onMounted, ref } from "vue"
import { useRouter } from "vue-router"
import DialogContent from "../components/DialogContent.vue"
import { useCentrifuge } from "../composables/useCentrifugo"
import { useSnackbar } from "../composables/useSnackbar"
import { CentrifugeSubscriptionType } from "../enums/centrifuge-subscription-type.enum"
import type { FactoryResetKeys } from "../types"

const { subscribe, history, onConnected } = useCentrifuge()
const { showError, snackbarState } = useSnackbar()
const router = useRouter()
const selectedFactoryResetKeys: Ref<string[]> = ref([])
const factoryResetDialog = ref(false)
const rebootDialog = ref(false)
const factoryResetKeys: Ref<FactoryResetKeys | undefined> = ref(undefined)

const factoryResetPayload = computed(() => {
	return {
		preserve: selectedFactoryResetKeys.value
	}
})

const emit = defineEmits<{
	(event: "rebootInProgress"): void
	(event: "factoryResetInProgress"): void
}>()

const showSuccess = (successMsg: string) => {
	snackbarState.msg = successMsg
	snackbarState.color = "success"
	snackbarState.timeout = 2000
	snackbarState.snackbar = true
}

const {
	onFetchError: onRebootError,
	error: rebootError,
	statusCode: rebootStatusCode,
	onFetchResponse: onRebootSuccess,
	execute: reboot,
	isFetching: rebootFetching
} = useFetch("reboot", { immediate: false }).post()

const {
	onFetchError: onResetError,
	error: resetError,
	statusCode: resetStatusCode,
	onFetchResponse: onResetSuccess,
	execute: reset,
	isFetching: resetFetching
} = useFetch("factory-reset", { immediate: false }).post(factoryResetPayload)

const {
	onFetchError: onReloadNetworkError,
	error: reloadNetworkError,
	statusCode: reloadNetworkStatusCode,
	onFetchResponse: onReloadNetworkSuccess,
	execute: reloadNetwork,
	isFetching: reloadNetworkLoading
} = useFetch("reload-network", { immediate: false }).post()

const loading = computed(() => rebootFetching.value || resetFetching.value || reloadNetworkLoading.value)

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

onRebootSuccess(() => {
	emit("rebootInProgress")
	rebootDialog.value = false
})

onRebootError(() => {
	if (rebootStatusCode.value === 401) {
		router.push("/login")
	} else {
		showError(`Rebooting device failed: ${JSON.stringify(rebootError.value)}`)
	}
})

onResetSuccess(() => {
	emit("factoryResetInProgress")
	factoryResetDialog.value = false
})

onResetError(() => {
	if (resetStatusCode.value === 401) {
		router.push("/login")
	} else {
		showError(`Resetting device failed: ${JSON.stringify(resetError.value)}`)
	}
})

const updateFactoryResetKeys = (data: FactoryResetKeys) => {
	factoryResetKeys.value = data
}

const loadHistoryAndSubscribe = () => {
	history(updateFactoryResetKeys, CentrifugeSubscriptionType.FactoryResetKeys)
	subscribe(updateFactoryResetKeys, CentrifugeSubscriptionType.FactoryResetKeys)
}

onConnected(() => {
	loadHistoryAndSubscribe()
})

onMounted(() => {
	loadHistoryAndSubscribe()
})
</script>

<template>
	<div class="flex flex-col gap-y-4 items-start">
		<div class="text-h4 text-secondary border-b w-100">Commands</div>
		<v-btn :prepend-icon="'mdi-refresh'" variant="text" :loading="reloadNetworkLoading" :disabled="loading"
			@click="reloadNetwork(false)">
			Restart network
		</v-btn>
		<v-btn :prepend-icon="'mdi-restart'" variant="text">
			Reboot
			<v-dialog v-model="rebootDialog" activator="parent" max-width="340" :no-click-animation="true" persistent
				@keydown.esc="rebootDialog = false">
				<DialogContent title="Reboot device" dialog-type="default" :show-close="true"
					@close="rebootDialog = false">
					<div class="flex flex-col gap-2 mb-8">
						Do you really want to restart the device?
					</div>
					<div class="flex justify-end -mr-4 mt-4">
						<v-btn variant="text" color="warning" :loading="loading" :disabled="loading"
							@click="reboot(false)">Reboot</v-btn>
						<v-btn variant="text" color="primary" @click="rebootDialog = false">Cancel</v-btn>
					</div>
				</DialogContent>
			</v-dialog>
		</v-btn>
		<v-btn :prepend-icon="'mdi-undo-variant'" variant="text">
			Factory Reset
			<v-dialog v-model="factoryResetDialog" activator="parent" max-width="340" :no-click-animation="true"
				persistent @keydown.esc="factoryResetDialog = false">
				<DialogContent title="Factory reset" dialog-type="default" :show-close="true"
					@close="factoryResetDialog = false">
					<div class="flex flex-col gap-2 mb-8">
						<v-checkbox-btn v-for="(option, index) in factoryResetKeys?.keys" :label="option"
							v-model="selectedFactoryResetKeys" :value="option" :key="index"></v-checkbox-btn>
					</div>
					<div class="flex justify-end -mr-4 mt-4">
						<v-btn variant="text" color="error" :loading="loading" :disabled="loading"
							@click="reset(false)">Reset</v-btn>
						<v-btn variant="text" color="primary" @click="factoryResetDialog = false">Cancel</v-btn>
					</div>
				</DialogContent>
			</v-dialog>
		</v-btn>
	</div>
</template>

<style scoped></style>