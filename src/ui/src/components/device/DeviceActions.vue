<script setup lang="ts">
import { computed, ref } from "vue"
import DialogContent from "../DialogContent.vue"
import { useCore } from "../../composables/useCore"
import { useCoreInitialization } from "../../composables/useCoreInitialization"
import { useAsyncAction } from "../../composables/useAsyncAction"
import { useDialogState } from "../../composables/useDialogState"

const { viewModel, reboot, factoryReset } = useCore()
const selectedFactoryResetKeys = ref<string[]>([])

useCoreInitialization()

const { dialogs, closeAll } = useDialogState<'reboot' | 'factoryReset'>()
const { loading, execute } = useAsyncAction({
	onSuccess: closeAll
})

const factoryResetKeys = computed(() => viewModel.factory_reset)

const handleReboot = () => execute(reboot)

const handleFactoryReset = () => execute(async () => {
	await factoryReset("1", selectedFactoryResetKeys.value)
})
</script>

<template>
	<div class="flex flex-col gap-y-4 items-start">
		<div class="text-h4 text-secondary border-b w-100">Commands</div>
		<v-btn :prepend-icon="'mdi-restart'" variant="text">
			Reboot
			<v-dialog v-model="dialogs.reboot" activator="parent" max-width="340" :no-click-animation="true" persistent
				@keydown.esc="dialogs.reboot = false">
				<DialogContent title="Reboot device" dialog-type="default" :show-close="true"
					@close="dialogs.reboot = false">
					<div class="flex flex-col gap-2 mb-8">
						Do you really want to restart the device?
					</div>
					<div class="flex justify-end -mr-4 mt-4">
						<v-btn variant="text" color="warning" :loading="loading" :disabled="loading"
							@click="handleReboot">Reboot</v-btn>
						<v-btn variant="text" color="primary" @click="dialogs.reboot = false">Cancel</v-btn>
					</div>
				</DialogContent>
			</v-dialog>
		</v-btn>
		<v-btn :prepend-icon="'mdi-undo-variant'" variant="text">
			Factory Reset
			<v-dialog v-model="dialogs.factoryReset" activator="parent" max-width="340" :no-click-animation="true"
				persistent @keydown.esc="dialogs.factoryReset = false">
				<DialogContent title="Factory reset" dialog-type="default" :show-close="true"
					@close="dialogs.factoryReset = false">
					<div class="flex flex-col gap-2 mb-8">
						<div v-if="factoryResetKeys?.keys && factoryResetKeys.keys.length > 0">
							<v-checkbox-btn v-for="(option, index) in factoryResetKeys.keys" :label="option"
								v-model="selectedFactoryResetKeys" :value="option" :key="index"></v-checkbox-btn>
						</div>
						<div v-else class="text-grey">
							No preserve options available
						</div>
					</div>
					<div class="flex justify-end -mr-4 mt-4">
						<v-btn variant="text" color="error" :loading="loading" :disabled="loading"
							@click="handleFactoryReset">Reset</v-btn>
						<v-btn variant="text" color="primary" @click="dialogs.factoryReset = false">Cancel</v-btn>
					</div>
				</DialogContent>
			</v-dialog>
		</v-btn>
	</div>
</template>

<style scoped></style>