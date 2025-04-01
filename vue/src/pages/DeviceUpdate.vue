<script setup lang="ts">
import { useFetch } from "@vueuse/core"
import { onMounted, ref } from "vue"
import DialogContent from "../components/DialogContent.vue"
import UpdateFileUpload from "../components/UpdateFileUpload.vue"
import UpdateInfo from "../components/UpdateInfo.vue"
import { useCentrifuge } from "../composables/useCentrifugo"
import { useOverlaySpinner } from "../composables/useOverlaySpinner"
import { useSnackbar } from "../composables/useSnackbar"
import { CentrifugeSubscriptionType } from "../enums/centrifuge-subscription-type.enum"
import router from "../plugins/router"
import type { SystemInfo } from "../types"
import type { UpdateValidationStatus } from "../types/update-validation-status"

const { updateDone } = useOverlaySpinner()
const { showError } = useSnackbar()
const { history } = useCentrifuge()

const updateStatus = ref("Recovered")
const updateDoneDialog = ref(false)
const currentVersion = ref<string>()
const loadUpdatePayload = ref({
	update_file_path: ""
})

const {
	onFetchError: onLoadUpdateError,
	error: loadUpdateError,
	statusCode: loadUpdateStatusCode,
	execute: loadUpdate,
	isFetching: loadUpdateFetching,
	response,
	data
} = useFetch("update/load", { immediate: false }).post(loadUpdatePayload).json()

onLoadUpdateError(async () => {
	if (loadUpdateStatusCode.value === 401) {
		router.push("/login")
	} else {
		showError(`Uploading file failed: ${(await response.value?.text()) ?? loadUpdateError.value}`)
	}
})

const loadUpdateData = (filename: string) => {
	loadUpdatePayload.value = {
		update_file_path: filename
	}
	loadUpdate(false)
}

const checkUpdateState = (data: UpdateValidationStatus) => {
	updateStatus.value = data.status
}

onMounted(() => {
	updateDone.on(() => {
		data.value = undefined
		history(checkUpdateState, CentrifugeSubscriptionType.UpdateStatus)
		updateDoneDialog.value = true
	})

	history((data: SystemInfo) => {
		currentVersion.value = data.os.version
	}, CentrifugeSubscriptionType.SystemInfo)
})
</script>

<template>
	<v-dialog persistent v-model="updateDoneDialog" width="25vw" :no-click-animation="true">
		<DialogContent v-if="updateStatus === 'Succeeded'" title="Update done" dialog-type="default"
			:show-close="false">
			<div class="flex flex-col gap-2 mb-8 items-center">
				<v-icon size="128" icon="mdi-checkbox-marked-circle-outline" color="success"></v-icon>
				<strong class="text-xl">Update installed successfully</strong>
			</div>
			<div class="flex justify-end -mr-4 mt-4">
				<v-btn variant="text" color="primary" @click="updateDoneDialog = false">OK</v-btn>
			</div>
		</DialogContent>
		<DialogContent v-else title="Update failed" dialog-type="Warning" :show-close="false">
			<div class="flex flex-col gap-2 mb-8 items-center">
				<v-icon size="128" icon="mdi-alert-circle-outline" color="warning"></v-icon>
				<strong class="text-xl">Update installation failed and recovered to previous version.</strong>
			</div>
			<div class="flex justify-end -mr-4 mt-4">
				<v-btn variant="text" color="primary" @click="updateDoneDialog = false">OK</v-btn>
			</div>
		</DialogContent>
	</v-dialog>
	<v-sheet :border="true" rounded class="m-20">
		<v-row class="m-8">
			<v-col sm="12" xl="6">
				<UpdateFileUpload @file-uploaded="loadUpdateData" />
			</v-col>
			<v-col sm="12" xl="6">
				<UpdateInfo :update-manifest="data" :load-update-fetching="loadUpdateFetching"
					:current-version="currentVersion" @reload-update-info="loadUpdate(false)" />
			</v-col>
		</v-row>
	</v-sheet>
</template>