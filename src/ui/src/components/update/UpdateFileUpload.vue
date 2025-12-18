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
const uploadState = computed(() => viewModel.firmware_upload_state)
const isUploading = computed(() => uploadState.value?.type === 'uploading')

watch(
	() => viewModel.device_operation_state,
	(state) => {
		if (state.type === 'reconnection_successful' && state.operation === 'Update') {
			updateFile.value = undefined
		}
	},
	{ deep: true }
)

const uploadFile = async () => {
	if (!viewModel.is_authenticated) {
		router.push("/login")
		return
	}

	if (!updateFile.value) {
		showError("Select an update file.")
		return
	}

	if (updateFile.value.type !== "application/x-tar") {
		showError("Wrong file type. Only tar archives are allowed.")
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
	<v-form @submit.prevent="uploadFile" enctype="multipart/form-data">
		<v-file-upload icon="mdi-file-upload" v-model="updateFile" clearable density="default"
			:disabled="isUploading">
			<template #item="{ file, props }">
				<v-file-upload-item v-bind="props">
					<template #title>
						<div class="flex justify-between">
							<div>{{ file.name }}</div>
						</div>
					</template>
				</v-file-upload-item>
			</template>
		</v-file-upload>
		<v-btn type="submit" prepend-icon="mdi-file-upload-outline" variant="text"
			:disabled="!updateFile || isUploading" class="mt-4">Upload</v-btn>
	</v-form>
</template>