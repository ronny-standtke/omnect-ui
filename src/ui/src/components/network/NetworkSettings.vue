<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue"
import { useSnackbar } from "../../composables/useSnackbar"
import { useCore } from "../../composables/useCore"
import { useClipboard } from "../../composables/useClipboard"
import { useIPValidation } from "../../composables/useIPValidation"
import type { DeviceNetwork } from "../../types"
import type { NetworkConfigRequest } from "../../composables/useCore"

const { showError } = useSnackbar()
const { viewModel, setNetworkConfig, networkFormReset, networkFormUpdate } = useCore()
const { copy } = useClipboard()
const { isValidIp: validateIp, parseNetmask } = useIPValidation()

const props = defineProps<{
    networkAdapter: DeviceNetwork
    isCurrentConnection: boolean
}>()

const ipAddress = ref(props.networkAdapter?.ipv4?.addrs[0]?.addr || "")
const dns = ref(props.networkAdapter?.ipv4?.dns?.join("\n") || "")
const gateways = ref(props.networkAdapter?.ipv4?.gateways?.join("\n") || "")
const addressAssignment = ref(props.networkAdapter?.ipv4?.addrs[0]?.dhcp ? "dhcp" : "static")
const netmask = ref(props.networkAdapter?.ipv4?.addrs[0]?.prefix_len || 24)

// State flags - declared early since they're used in watchers and initialization
const isSubmitting = ref(false)
const isSyncingFromWebSocket = ref(false)
const isStartingFreshEdit = ref(false)

// NOTE: NetworkFormStartEdit is now called by the parent DeviceNetworks.vue when tab changes
// This prevents all mounted components from calling it simultaneously
// Set flag to prevent the dirty flag watch from resetting form during initialization
isStartingFreshEdit.value = true
nextTick(() => {
    nextTick(() => {
        isStartingFreshEdit.value = false
    })
})

// Helper to reset form fields to match current adapter data
const resetFormFields = () => {
    ipAddress.value = props.networkAdapter?.ipv4?.addrs[0]?.addr || ""
    dns.value = props.networkAdapter?.ipv4?.dns?.join("\n") || ""
    gateways.value = props.networkAdapter?.ipv4?.gateways?.join("\n") || ""
    addressAssignment.value = props.networkAdapter?.ipv4?.addrs[0]?.dhcp ? "dhcp" : "static"
    netmask.value = props.networkAdapter?.ipv4?.addrs[0]?.prefix_len || 24
}

// Helper to send current form data to Core for dirty flag tracking
const sendFormUpdateToCore = () => {
    const formData = {
        name: props.networkAdapter.name,
        ip_address: ipAddress.value,
        dhcp: addressAssignment.value === "dhcp",
        prefix_len: netmask.value,
        dns: dns.value.split("\n").filter(d => d.trim()),
        gateways: gateways.value.split("\n").filter(g => g.trim())
    }
    networkFormUpdate(JSON.stringify(formData))
}

// Watch form fields and notify Core when they change
// Use flush: 'post' to ensure watcher runs after all DOM updates
watch([ipAddress, dns, gateways, addressAssignment, netmask], () => {
    // Don't update dirty flag during submit or WebSocket sync
    // Note: Core validates adapter_name matches the currently editing adapter
    // This defends against hidden components (v-window) sending stale data
    if (!isSubmitting.value && !isSyncingFromWebSocket.value) {
        sendFormUpdateToCore()
    }
}, { flush: 'post' })

// Watch for form reset from Core (when dirty flag clears for this adapter)
// This should ONLY reset the form when the user explicitly clicks "Reset",
// NOT when starting a fresh edit or after completing a submit
watch(() => viewModel.network_form_dirty, (isDirty, wasDirty) => {
    // Skip reset if we're starting a fresh edit (the form is being initialized)
    if (isStartingFreshEdit.value) {
        return
    }

    // Skip reset during submit - the dirty flag clears during submit, not because user reset
    if (isSubmitting.value) {
        return
    }

    const isEditingThisAdapter = viewModel.network_form_state?.type === 'editing' &&
                                  (viewModel.network_form_state as any).adapter_name === props.networkAdapter.name

    if (wasDirty === true && isDirty === false && isEditingThisAdapter) {
        // Reset local form state to match current adapter
        isSyncingFromWebSocket.value = true
        resetFormFields()

        nextTick(() => {
            nextTick(() => {
                isSyncingFromWebSocket.value = false
            })
        })
    }
})

// Watch for prop changes from WebSocket updates and sync local state
// IMPORTANT: We watch the entire adapter object to ensure reactivity,
// but only reset form fields if not dirty. This allows props.networkAdapter.online
// to remain reactive even when the form is dirty.
watch(() => props.networkAdapter, (newAdapter) => {
    if (!newAdapter) return

    // Only reset form fields if user hasn't made unsaved changes
    // But we still need to let this watcher run to maintain reactivity for
    // non-form props like 'online' status
    if (!isSubmitting.value && !viewModel.network_form_dirty) {
        // Set flag to prevent form watchers from firing during sync
        isSyncingFromWebSocket.value = true
        resetFormFields()

        // Clear flag after Vue finishes all reactive updates AND all post-flush watchers
        // Need double nextTick: first for reactive updates, second for post-flush watchers
        nextTick(() => {
            nextTick(() => {
                isSyncingFromWebSocket.value = false
            })
        })
    }
    // Note: We don't return early when dirty - this allows Vue's reactivity
    // system to track changes to props.networkAdapter.online and other non-form props
}, { deep: true })

const isDHCP = computed(() => addressAssignment.value === "dhcp")

// Use Core's computed rollback modal flags
const isRollbackRequired = computed(() => viewModel.should_show_rollback_modal)
const enableRollback = ref(true) // Tracks user's checkbox state
const confirmationModalOpen = ref(false)

// Watch Core's default_rollback_enabled to update checkbox when modal shows
watch(() => viewModel.should_show_rollback_modal, (shouldShow) => {
    if (shouldShow) {
        enableRollback.value = viewModel.default_rollback_enabled
    }
})

// Determine if switching to DHCP for UI text (Core computes this, but we still need it for modal text)
const switchingToDhcp = computed(() => !props.networkAdapter?.ipv4?.addrs[0]?.dhcp && isDHCP.value)

const restoreSettings = () => {
    // Reset Core state (clears dirty flag and NetworkFormState)
    networkFormReset(props.networkAdapter.name)

    // Reset local form state
    resetFormFields()
}

const setNetMask = (mask: string) => {
    const prefixLen = parseNetmask(mask)
    if (prefixLen === null) {
        return "Invalid netmask"
    }
    netmask.value = prefixLen
}

watch(
	() => viewModel.error_message,
	(newMessage) => {
		if (newMessage) {
			showError(newMessage)
			isSubmitting.value = false
		}
	}
)

watch(
	() => viewModel.success_message,
	(newMessage) => {
		if (newMessage) {
			isSubmitting.value = false
            confirmationModalOpen.value = false
		}
	}
)

// Watch for form state changes from Core as a reliable way to reset submitting state
watch(
	() => viewModel.network_form_state,
	(newState) => {
		// If we were submitting and the state is no longer submitting, reset our flag
		if (isSubmitting.value && newState?.type !== 'submitting') {
			isSubmitting.value = false
		}
	}
)

const submit = async () => {
    // Check if the change requires rollback protection
    if (isRollbackRequired.value) {
        confirmationModalOpen.value = true
    } else {
        await submitNetworkConfig(false)
    }
}

const submitNetworkConfig = async (includeRollback: boolean) => {
    isSubmitting.value = true
    confirmationModalOpen.value = false

    const config: NetworkConfigRequest = {
        isServerAddr: props.isCurrentConnection,
        ipChanged: props.networkAdapter.ipv4?.addrs[0]?.addr !== ipAddress.value,
        name: props.networkAdapter.name,
        dhcp: isDHCP.value,
        ip: ipAddress.value || null,
        previousIp: props.networkAdapter.ipv4?.addrs[0]?.addr || null,
        netmask: netmask.value || null,
        gateway: gateways.value.split("\n").filter(g => g.trim()) || [],
        dns: dns.value.split("\n").filter(d => d.trim()) || [],
        enableRollback: includeRollback ? enableRollback.value : null,
        switchingToDhcp: switchingToDhcp.value
    }

    await setNetworkConfig(JSON.stringify(config))
}

const cancelRollbackModal = () => {
    confirmationModalOpen.value = false
}
</script>

<template>
    <div>
        <!-- Rollback Confirmation Modal -->
        <v-dialog v-model="confirmationModalOpen" max-width="600">
            <v-card>
                <v-card-title class="text-h5">
                    Confirm Network Configuration Change
                </v-card-title>
                <v-card-text>
                    <v-alert type="warning" variant="tonal" class="mb-4">
                        This change will disconnect your current session.
                    </v-alert>
                    <p class="mb-4">
                        <template v-if="switchingToDhcp">
                            You are about to switch to DHCP on the network adapter you're currently connected to.
                            This will likely assign a new IP address and interrupt your connection.
                        </template>
                        <template v-else>
                            You are about to change the IP address of the network adapter you're currently connected to.
                            This will interrupt your connection.
                        </template>
                    </p>
          <v-checkbox
            v-model="enableRollback"
            :label="switchingToDhcp ? 'Enable automatic rollback' : 'Enable automatic rollback (recommended)'"
            hide-details
          >
            <template v-slot:label>
              <strong>{{ switchingToDhcp ? 'Enable automatic rollback' : 'Enable automatic rollback (recommended)' }}</strong>
            </template>
          </v-checkbox>
          <div class="text-caption text-medium-emphasis ml-8">
            <template v-if="switchingToDhcp">
              Not recommended for DHCP: You won't know the new IP address, making it difficult to confirm the change before the 90 second timeout triggers a rollback.
            </template>
            <template v-else>
              If you can't reach the new IP and log in within 90 seconds, the device will automatically restore the previous configuration.
            </template>
          </div>
                </v-card-text>
                <v-card-actions>
                    <v-spacer></v-spacer>
                    <v-btn color="secondary" variant="text" @click="cancelRollbackModal">
                        Cancel
                    </v-btn>
                    <v-btn color="primary" variant="text" @click="submitNetworkConfig(true)">
                        Apply Changes
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>

        <v-form @submit.prevent="submit" class="flex flex-col gap-y-4 ml-4">
            <v-chip size="large" class="ma-2" label
                :color="props.networkAdapter.online ? 'light-green-darken-2' : 'red-darken-2'">
                {{ props.networkAdapter.online ? "Online" : "Offline" }}{{ props.isCurrentConnection && props.networkAdapter.online ? " (current connection)" : "" }}
            </v-chip>
            <v-radio-group v-model="addressAssignment" inline>
                <v-radio label="DHCP" value="dhcp"></v-radio>
                <v-radio label="Static" value="static"></v-radio>
            </v-radio-group>
            <v-text-field :readonly="isDHCP" v-model="ipAddress" label="IP Address" :rules="[validateIp]" outlined
                append-inner-icon="mdi-content-copy" @click:append-inner="copy(`${ipAddress}/${netmask}`)">
                <template #append-inner>
                    <v-btn :disabled="isDHCP" size="large" append-icon="mdi-menu-down" variant="text" density="compact"
                        slim class="m-0">
                        /{{ netmask }}
                        <v-menu activator="parent">
                            <v-list>
                                <v-list-item v-for="(item, index) in ['/8', '/16', '/24', '/32']" :key="index"
                                    :value="index">
                                    <v-list-item-title @click="setNetMask(item)">{{ item }}</v-list-item-title>
                                </v-list-item>
                            </v-list>
                        </v-menu>
                    </v-btn>
                </template>
            </v-text-field>
            <v-text-field label="MAC Address" variant="outlined" readonly v-model="props.networkAdapter.mac"
                append-inner-icon="mdi-content-copy"
                @click:append-inner="copy(props.networkAdapter.mac)"></v-text-field>
            <v-textarea :readonly="isDHCP" v-model="gateways" label="Gateways" variant="outlined" rows="3" no-resize
                append-inner-icon="mdi-content-copy" @click:append-inner="copy(ipAddress)"></v-textarea>
            <v-textarea v-model="dns" label="DNS" variant="outlined" rows="3" no-resize
                append-inner-icon="mdi-content-copy" @click:append-inner="copy(ipAddress)"></v-textarea>
            <div class="flex flex-row gap-x-4">
                <v-btn color="secondary" type="submit" variant="text" :loading="isSubmitting">
                    Save
                </v-btn>
                <v-btn :disabled="isSubmitting" type="reset" variant="text" @click.prevent="restoreSettings">
                    Reset
                </v-btn>
            </div>
        </v-form>
    </div>
</template>

<style lang="css">
.v-field:has(input[type="text"]:read-only),
.v-field:has(textarea:read-only) {
    background-color: #f5f5f5 !important;
}
</style>
