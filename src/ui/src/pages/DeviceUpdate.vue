<script setup lang="ts">
import { computed } from "vue"
import UpdateFileUpload from "../components/update/UpdateFileUpload.vue"
import UpdateInfo from "../components/update/UpdateInfo.vue"
import { useCore } from "../composables/useCore"
import { useCoreInitialization } from "../composables/useCoreInitialization"
import { useMessageWatchers } from "../composables/useMessageWatchers"

const { viewModel, loadUpdate } = useCore()

useCoreInitialization()
useMessageWatchers()

const currentVersion = computed(() => viewModel.systemInfo?.os?.version)

const loadUpdateFetching = computed(() => viewModel.isLoading)

const loadUpdateData = (filename?: string) => {
	// The backend uses a fixed path regardless of the filename
	loadUpdate(filename ?? "")
}
</script>

<template>
	<v-sheet :border="true" rounded class="ma-4">
		<v-row class="ma-4">
			<v-col cols="12">
				<UpdateFileUpload @file-uploaded="loadUpdateData" />
			</v-col>
			<v-col cols="12">
				<UpdateInfo :update-manifest="viewModel.updateManifest" :load-update-fetching="loadUpdateFetching"
					:current-version="currentVersion" @reload-update-info="loadUpdateData" />
			</v-col>
		</v-row>
	</v-sheet>
</template>