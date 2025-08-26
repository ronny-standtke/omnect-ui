<script setup lang="ts">
import { useFetch } from "@vueuse/core"
import { ref, toRef } from "vue"
import { useOverlaySpinner } from "../composables/useOverlaySpinner"
import { useSnackbar } from "../composables/useSnackbar"
import { useWaitReconnect } from "../composables/useWaitReconnect"
import router from "../plugins/router"
import type { UpdateManifest } from "../types/update-manifest"
import KeyValuePair from "./ui-components/KeyValuePair.vue"

const { showError: snackbarShowError } = useSnackbar()
const { overlaySpinnerState, reset: resetOverlay } = useOverlaySpinner()
const { startWaitReconnect, stopWaitReconnect } = useWaitReconnect()

const props = defineProps<{
	updateManifest: UpdateManifest | undefined
	currentVersion: string | undefined
	loadUpdateFetching: boolean
}>()

defineEmits<(event: "reloadUpdateInfo") => void>()

const updateManifest = toRef(props, "updateManifest")
const runUpdatePayload = ref<{ validate_iothub_connection: boolean }>({ validate_iothub_connection: false })

const {
	onFetchError: onRunUpdateError,
	error: runUpdateError,
	statusCode: runUpdateStatusCode,
	execute: runUpdate,
	response
} = useFetch("update/run", { immediate: false }).post(runUpdatePayload)

onRunUpdateError(async () => {
	if (runUpdateStatusCode.value === 401) {
		router.push("/login")
	} else {
		showError(`Running update failed: ${(await response.value?.text()) ?? runUpdateError.value}`)
	}
})

const triggerUpdate = () => {
	runUpdate(false)
	overlaySpinnerState.title = "Installing update"
	overlaySpinnerState.text = "Please have some patience, the update may take some time."
	overlaySpinnerState.overlay = true
	overlaySpinnerState.isUpdateRunning = true
	startWaitReconnect()
}

const showError = (errorMsg: string) => {
	resetOverlay()
	snackbarShowError(errorMsg)
	stopWaitReconnect()
}

const toggleEnforceConnect = (v: boolean | null) => {
	if (!v) return
	runUpdatePayload.value = {
		validate_iothub_connection: v
	}
}
</script>

<template>
	<div class="flex flex-col gap-y-8">
		<div class="flex border-b gap-x-4 items-center">
			<div class="text-h4 text-secondary">Update Info</div>
			<v-btn prepend-icon="mdi-reload" :disabled="!updateManifest" :loading="loadUpdateFetching"
				@click="$emit('reloadUpdateInfo')" variant="text">Load update Info</v-btn>
		</div>
		<dl v-if="updateManifest" class="grid grid-cols-[1fr_3fr] gap-x-64 gap-y-8">
			<KeyValuePair title="Provider">{{ updateManifest.updateId.provider }}</KeyValuePair>
			<KeyValuePair title="omnect Secure OS variant">{{ updateManifest.updateId.name }}</KeyValuePair>
			<KeyValuePair title="Current omnect Secure OS version">{{ props.currentVersion }}</KeyValuePair>
			<KeyValuePair title="Update omnect Secure OS version">{{ updateManifest.updateId.version }}</KeyValuePair>
			<KeyValuePair title="Manufacturer">{{ updateManifest.compatibility[0].manufacturer }}</KeyValuePair>
			<KeyValuePair title="Model">{{ updateManifest.compatibility[0].model }}</KeyValuePair>
			<KeyValuePair title="Compatibility Id">{{ updateManifest.compatibility[0].compatibilityid }}</KeyValuePair>
			<KeyValuePair title="Created">{{ updateManifest.createdDateTime ? new
				Date(updateManifest.createdDateTime).toLocaleString() : "" }}</KeyValuePair>
		</dl>
	</div>
	<div v-if="updateManifest" class="flex flex-col mt-4 items-start">
		<v-checkbox v-bind="props" v-model:model-value="runUpdatePayload.validate_iothub_connection"
			@update:model-value="toggleEnforceConnect" label="Enforce omnect cloud connection"
			hint="If checkbox is enabled, an update will be considered successful only if afterward the device is able to establish a connection to the omnect cloud (Azure IoT Hub)."
			persistent-hint density="compact"></v-checkbox>
		<v-btn class="mt-4" prepend-icon="mdi-update" variant="text" @click="triggerUpdate()">Install
			update</v-btn>
	</div>
</template>