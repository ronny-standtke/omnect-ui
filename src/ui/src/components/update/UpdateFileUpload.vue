<script setup lang="ts">
import axios, { AxiosError } from "axios"
import { computed, ref, watch } from "vue"
import { useCore } from "../../composables/useCore"
import { useSnackbar } from "../../composables/useSnackbar"
import router from "../../plugins/router"
import { DeviceEventVariantUploadStarted, DeviceEventVariantUploadProgress, DeviceEventVariantUploadCompleted, DeviceEventVariantUploadFailed, EventVariantDevice } from "../../../../shared_types/generated/typescript/types/shared_types"

const { showError } = useSnackbar()
const { viewModel, sendEvent } = useCore()
const emit = defineEmits<(e: "fileUploaded", filename: string) => void>()

const updateFile = ref<File>()

// Derived state from Core
const uploadState = computed(() => viewModel.firmwareUploadState)
const isUploading = computed(() => uploadState.value?.type === 'uploading')

watch(
	() => viewModel.deviceOperationState,
	(state) => {
		if (state.type === 'reconnectionSuccessful' && state.operation === 'Update') {
			updateFile.value = undefined
		}
	},
	{ deep: true }
)

// Auto-upload when file is selected
watch(updateFile, (newFile) => {
	if (newFile) {
		uploadFile()
	}
})

const replaceFile = () => {
	updateFile.value = undefined
}

const uploadFile = async () => {
	if (!viewModel.isAuthenticated) {
		router.push("/login")
		return
	}

	if (!updateFile.value) {
		return
	}

	if (updateFile.value.type !== "application/x-tar") {
		showError("Wrong file type. Only tar archives are allowed.")
		updateFile.value = undefined // Reset if invalid
		return
	}

	const formData = new FormData()
	formData.append("file", updateFile.value as File)

	// Notify Core: Upload Started
	sendEvent(new EventVariantDevice(new DeviceEventVariantUploadStarted()))

	try {
		const res = await axios.post("update/file", formData, {
			withCredentials: true,
			onUploadProgress({ progress }) {
				const percentage = progress ? Math.ceil(progress * 100) : 0
				// Notify Core: Upload Progress
				sendEvent(new EventVariantDevice(new DeviceEventVariantUploadProgress(percentage)))
			},
			responseType: "text"
		})

		if (res.status < 300) {
			// Notify Core: Upload Completed
			sendEvent(new EventVariantDevice(new DeviceEventVariantUploadCompleted(updateFile.value.name)))
			emit("fileUploaded", updateFile.value.name)
		} else if (res.status === 401) {
			router.push("/login")
		} else {
			const errorMsg = `Uploading file failed: ${res.data}`
			showError(errorMsg)
			// Notify Core: Upload Failed
			sendEvent(new EventVariantDevice(new DeviceEventVariantUploadFailed(errorMsg)))
		}
	} catch (err) {
		const errorMsg = `Uploading file failed: ${err as AxiosError}`
		showError(errorMsg)
		// Notify Core: Upload Failed
		sendEvent(new EventVariantDevice(new DeviceEventVariantUploadFailed(errorMsg)))
	}

	formData.delete("file")
}
</script>

<template>
	<v-form enctype="multipart/form-data">
		<!-- Drop Zone (Visible when no file is selected) -->
		<v-file-upload v-if="!updateFile" icon="mdi-file-upload" v-model="updateFile" clearable density="compact"
			title="Drag and drop update file here" class="update-drop-zone" :disabled="isUploading">
		</v-file-upload>

		<!-- Compact File Card (Visible when file is selected) -->
		<v-card v-else variant="outlined" class="d-flex align-center pa-4 bg-surface-light">
			<v-icon icon="mdi-file-check" color="success" class="mr-4" size="large"></v-icon>
			<div class="flex-grow-1">
				<div class="text-subtitle-1 font-weight-medium">{{ updateFile.name }}</div>
				<div class="text-caption text-medium-emphasis">
					{{ (updateFile.size / 1024 / 1024).toFixed(2) }} MB
					<span v-if="isUploading"> - Uploading...</span>
				</div>
			</div>
			<v-btn variant="text" color="primary" size="small" @click="replaceFile" :disabled="isUploading">
				Replace File
			</v-btn>
		</v-card>
	</v-form>
</template>

<style scoped>
/* Reduce the vertical height of the drop zone and change visual style */
:deep(.v-file-upload .v-input__control) {
	min-height: 120px !important;
	border-style: solid !important;
	background-color: rgba(var(--v-theme-on-surface), 0.04) !important;
}
</style>