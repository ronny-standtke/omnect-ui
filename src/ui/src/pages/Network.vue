<script setup lang="ts">
import { ref, watch } from "vue"
import { onBeforeRouteLeave, useRouter } from "vue-router"
import DeviceNetworks from "../components/network/DeviceNetworks.vue"
import { useCoreInitialization } from "../composables/useCoreInitialization"
import { useCore } from "../composables/useCore"

useCoreInitialization()

const { viewModel, networkFormReset } = useCore()
const router = useRouter()
const showNavigationDialog = ref(false)
let pendingRoute: any = null
const waitingForReset = ref(false)

// Watch for form reset completion (dirty flag clears)
watch(() => viewModel.network_form_dirty, (isDirty, wasDirty) => {
  // If we're waiting for reset and dirty flag changed from true to false, proceed with navigation
  if (waitingForReset.value && wasDirty === true && isDirty === false) {
    waitingForReset.value = false

    // Use programmatic navigation instead of trying to resume the blocked navigation
    if (pendingRoute) {
      router.push(pendingRoute)
      pendingRoute = null
    }
  }
})

// Navigation guard to prevent leaving page with unsaved changes
onBeforeRouteLeave((to, _from, next) => {
  if (viewModel.network_form_dirty === true) {
    showNavigationDialog.value = true
    pendingRoute = to // Save the destination route
    next(false) // Block navigation
  } else {
    next() // Allow navigation
  }
})

const confirmNavigation = () => {
  // User confirmed, discard changes and navigate
  const currentAdapter = viewModel.network_form_state?.type === 'editing'
    ? (viewModel.network_form_state as any).adapter_name
    : null

  showNavigationDialog.value = false

  if (currentAdapter) {
    // Set flag to wait for reset completion
    waitingForReset.value = true
    networkFormReset(currentAdapter)
    // Navigation will be triggered by the watcher when reset completes
  } else {
    // No adapter to reset, navigate immediately
    if (pendingRoute) {
      router.push(pendingRoute)
      pendingRoute = null
    }
  }
}

const cancelNavigation = () => {
  // User cancelled, stay on current page
  showNavigationDialog.value = false
  pendingRoute = null
  waitingForReset.value = false
}
</script>

<template>
    <v-sheet :border="true" rounded class="m-20">
        <div class="flex flex-col gap-y-16 m-8">
            <DeviceNetworks></DeviceNetworks>
        </div>
    </v-sheet>

    <!-- Unsaved changes confirmation dialog (page navigation) -->
    <v-dialog v-model="showNavigationDialog" max-width="500">
      <v-card>
        <v-card-title class="text-h5">Unsaved Changes</v-card-title>
        <v-card-text>
          You have unsaved changes. Do you want to discard them and leave this page?
        </v-card-text>
        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn color="primary" text @click="cancelNavigation">Cancel</v-btn>
          <v-btn color="error" text @click="confirmNavigation" data-cy="network-confirm-discard-button">Discard Changes</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
</template>