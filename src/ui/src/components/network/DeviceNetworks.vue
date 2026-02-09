<script setup lang="ts">
import { computed, ref, watch } from "vue"
import NetworkSettings from "./NetworkSettings.vue"
import { useCore } from "../../composables/useCore"
import { useCoreInitialization } from "../../composables/useCoreInitialization"

const { viewModel, networkFormReset, networkFormStartEdit } = useCore()

useCoreInitialization()

const tab = ref<string | null>(null)
const showUnsavedChangesDialog = ref(false)
const pendingTab = ref<string | null>(null)
const isReverting = ref(false)

const networkStatus = computed(() => viewModel.networkStatus)

// Use Core's computed current connection adapter
const isCurrentConnection = (adapter: any) => {
  return viewModel.currentConnectionAdapter === adapter.name
}

// Watch for tab changes and check for unsaved changes
watch(tab, (newTab, oldTab) => {
  if (newTab === oldTab) return

  // Skip if this is a programmatic revert
  if (isReverting.value) {
    isReverting.value = false
    return
  }

  // Check if there are unsaved changes
  if (viewModel.networkFormDirty && oldTab !== null) {
    // Block the tab change and show confirmation dialog
    showUnsavedChangesDialog.value = true
    pendingTab.value = newTab as string
    // Revert tab back to old tab
    isReverting.value = true
    tab.value = oldTab
  } else if (newTab) {
    // Tab change successful - notify Core to start editing this adapter
    networkFormStartEdit(newTab as string)
  }
})

const confirmTabChange = () => {
  if (pendingTab.value !== null) {
    // User confirmed, discard changes and switch tabs
    const currentAdapter = viewModel.networkFormState?.type === 'editing'
      ? (viewModel.networkFormState as any).adapterName
      : null

    if (currentAdapter) {
      networkFormReset(currentAdapter)
    }

    // Now switch to the pending tab
    tab.value = pendingTab.value

    // Start editing the new adapter
    networkFormStartEdit(pendingTab.value)

    pendingTab.value = null
  }
  showUnsavedChangesDialog.value = false
}

const cancelTabChange = () => {
  // User cancelled, stay on current tab
  pendingTab.value = null
  showUnsavedChangesDialog.value = false
}
</script>

<template>
  <div class="flex flex-col gap-y-4 flex-wrap">
    <div class="flex border-b gap-x-4 items-center">
      <div class="text-h4 text-secondary">Network</div>
    </div>
    <div class="d-flex flex-row">
      <v-tabs v-model="tab" color="primary" direction="vertical" class="border-r network-tabs">
        <v-tab v-for="networkAdapter in networkStatus?.networkStatus" :value="networkAdapter.name" class="text-none">
          <div class="d-flex align-center w-100 py-2">
            <v-icon 
              :icon="networkAdapter.online ? 'mdi-circle' : 'mdi-circle-outline'" 
              :color="networkAdapter.online ? 'success' : 'grey-lighten-1'"
              size="x-small" 
              class="mr-3"
            ></v-icon>
            <span class="font-weight-medium">{{ networkAdapter.name }}</span>
            <v-spacer></v-spacer>
            <v-icon v-if="isCurrentConnection(networkAdapter)" icon="mdi-account-network" size="x-small" color="info" class="ml-3" title="Current Connection"></v-icon>
          </div>
        </v-tab>
      </v-tabs>
      <v-window v-model="tab" class="flex-grow-1" direction="vertical">
        <v-window-item v-for="networkAdapter in networkStatus?.networkStatus" :key="networkAdapter.name" :value="networkAdapter.name">
          <NetworkSettings :networkAdapter="networkAdapter" :isCurrentConnection="isCurrentConnection(networkAdapter)" />
        </v-window-item>
      </v-window>
    </div>

    <!-- Unsaved changes confirmation dialog (tab switching) -->
    <v-dialog v-model="showUnsavedChangesDialog" max-width="500">
      <v-card>
        <v-card-title class="text-h5">Unsaved Changes</v-card-title>
        <v-card-text>
          You have unsaved changes. Do you want to discard them and switch to another network adapter?
        </v-card-text>
        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn color="primary" variant="text" @click="cancelTabChange">Cancel</v-btn>
          <v-btn color="error" variant="text" @click="confirmTabChange" data-cy="network-confirm-discard-button">Discard Changes</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.network-tabs {
  min-width: 180px;
}
</style>