<script setup lang="ts">
import { ref, toRef, type DeepReadonly } from "vue"
import { useCore } from "../../composables/useCore"
import type { UpdateManifest } from "../../composables/useCore"
import KeyValuePair from "../ui-components/KeyValuePair.vue"

const { runUpdate } = useCore()

const props = defineProps<{
	updateManifest: DeepReadonly<UpdateManifest> | null | undefined
	currentVersion: string | undefined
	loadUpdateFetching: boolean
}>()

defineEmits<(event: "reloadUpdateInfo") => void>()

const updateManifest = toRef(props, "updateManifest")
const runUpdatePayload = ref<{ validate_iothub_connection: boolean }>({ validate_iothub_connection: false })

const triggerUpdate = async () => {
	await runUpdate(runUpdatePayload.value.validate_iothub_connection)
}

const toggleEnforceConnect = (v: boolean | null) => {
	if (!v) return
	runUpdatePayload.value = {
		validate_iothub_connection: v
	}
}
</script>

<template>
	<div class="flex flex-col gap-y-6">
		<!-- Header -->
		<div class="flex border-b pb-2 items-center justify-between">
			<div class="text-h5 text-secondary font-weight-bold">Update Details</div>
			<!-- Optional reload button if needed, but removing main button as requested -->
		</div>

		<!-- Info Grid -->
		<div v-if="updateManifest" class="grid grid-cols-1 md:grid-cols-3 gap-6">
			<!-- Column 1: Version Info -->
			<div class="flex flex-col gap-2">
				<div class="text-subtitle-2 text-medium-emphasis mb-1">Version</div>
				<KeyValuePair title="Current Version">{{ props.currentVersion }}</KeyValuePair>
				<KeyValuePair title="Update Version">{{ updateManifest.updateId.version }}</KeyValuePair>
				<KeyValuePair title="Variant">{{ updateManifest.updateId.name }}</KeyValuePair>
			</div>

			<!-- Column 2: Provider Info -->
			<div class="flex flex-col gap-2">
				<div class="text-subtitle-2 text-medium-emphasis mb-1">Provider</div>
				<KeyValuePair title="Provider">{{ updateManifest.updateId.provider }}</KeyValuePair>
				<KeyValuePair title="Created">{{ updateManifest.createdDateTime ? new
					Date(updateManifest.createdDateTime).toLocaleString() : "" }}</KeyValuePair>
			</div>

			<!-- Column 3: Compatibility -->
			<div class="flex flex-col gap-2">
				<div class="text-subtitle-2 text-medium-emphasis mb-1">Compatibility</div>
				<KeyValuePair title="Manufacturer">{{ updateManifest.compatibility[0]?.manufacturer }}</KeyValuePair>
				<KeyValuePair title="Model">{{ updateManifest.compatibility[0]?.model }}</KeyValuePair>
				<KeyValuePair title="Compatibility Id">{{ updateManifest.compatibility[0]?.compatibilityid }}</KeyValuePair>
			</div>
		</div>

		<!-- Empty State / Placeholder -->
		<div v-else class="text-body-1 text-medium-emphasis py-8 text-center italic">
			Upload a file to see update details.
		</div>

		<!-- Actions Footer -->
		<div v-if="updateManifest" class="flex flex-col gap-4 mt-2 pt-4 border-t">
			<v-checkbox v-bind="props" v-model:model-value="runUpdatePayload.validate_iothub_connection"
				@update:model-value="toggleEnforceConnect" label="Enforce omnect cloud connection"
				hint="If checkbox is enabled, an update will be considered successful only if afterward the device is able to establish a connection to the omnect cloud (Azure IoT Hub)."
				persistent-hint density="compact" color="primary"></v-checkbox>
			
			<div class="flex justify-end">
				<v-btn prepend-icon="mdi-update" color="primary" variant="flat" size="large" @click="triggerUpdate()">
					Install Update
				</v-btn>
			</div>
		</div>
	</div>
</template>