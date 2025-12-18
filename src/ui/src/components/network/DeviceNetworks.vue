<script setup lang="ts">
import { computed, ref, watch } from "vue"
import NetworkSettings from "./NetworkSettings.vue"
import { useCore } from "../../composables/useCore"
import { useCoreInitialization } from "../../composables/useCoreInitialization"

const { viewModel, networkFormReset } = useCore()

useCoreInitialization()

const tab = ref<string | null>(null)
const showUnsavedChangesDialog = ref(false)
const pendingTab = ref<string | null>(null)
const isReverting = ref(false)

const networkStatus = computed(() => viewModel.network_status)

// Determine if an adapter is the current connection by comparing browser hostname with adapter IPs
const isCurrentConnection = (adapter: any) => {
  const hostname = window.location.hostname
  if (!adapter.ipv4?.addrs) return false

  // Check if any of the adapter's IPs match the browser's hostname
  const directMatch = adapter.ipv4.addrs.some((ip: any) => ip.addr === hostname)

  if (directMatch) {
    return true
  }

  // If hostname is not an IP (e.g., "omnect-device"), we can't determine which adapter
  // So mark the first online adapter with an IP as the current connection
  const isHostnameAnIP = /^(\d{1,3}\.){3}\d{1,3}$/.test(hostname)
  if (!isHostnameAnIP && adapter.online && adapter.ipv4.addrs.length > 0) {
    // Check if this is the first online adapter
    const allAdapters = networkStatus.value?.network_status || []
    const firstOnlineAdapter = allAdapters.find((a: any) => a.online && a.ipv4?.addrs?.length > 0)

    if (firstOnlineAdapter?.name === adapter.name) {
      return true
    }
  }

  return false
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
        <v-window-item v-for="networkAdapter in networkStatus?.network_status" :value="networkAdapter.name">
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