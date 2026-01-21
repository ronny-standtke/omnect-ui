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

const networkStatus = computed(() => viewModel.network_status)

// Use Core's computed current connection adapter
const isCurrentConnection = (adapter: any) => {
  return viewModel.current_connection_adapter === adapter.name
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
  if (viewModel.network_form_dirty && oldTab !== null) {
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
    const currentAdapter = viewModel.network_form_state?.type === 'editing'
      ? (viewModel.network_form_state as any).adapter_name
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
      <v-tabs v-model="tab" color="primary" direction="vertical">
        <v-tab v-for="networkAdapter in networkStatus?.network_status" :text="networkAdapter.name"
          :value="networkAdapter.name"></v-tab>
      </v-tabs>
      <v-window v-model="tab" class="w[20vw]" direction="vertical">
        <v-window-item v-for="networkAdapter in networkStatus?.network_status" :key="networkAdapter.name" :value="networkAdapter.name">
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
          <v-btn color="primary" text @click="cancelTabChange">Cancel</v-btn>
          <v-btn color="error" text @click="confirmTabChange">Discard Changes</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>