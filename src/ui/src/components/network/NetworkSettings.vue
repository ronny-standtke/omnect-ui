<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue"
import { useSnackbar } from "../../composables/useSnackbar"
import { useCore } from "../../composables/useCore"
import { useClipboard } from "../../composables/useClipboard"
import { useIPValidation } from "../../composables/useIPValidation"
import type { DeviceNetwork } from "../../types"

const { showError } = useSnackbar()
const { viewModel, setNetworkConfig, networkFormReset, networkFormUpdate, networkFormStartEdit } = useCore()
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

// Initialize form editing state in Core when component mounts
// Set flag to prevent the dirty flag watch from resetting form during initialization
isStartingFreshEdit.value = true
networkFormStartEdit(props.networkAdapter.name)
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
watch(() => props.networkAdapter, (newAdapter) => {
    if (!newAdapter) return

    // Don't overwrite user's unsaved changes during submit or when user has made edits
    if (isSubmitting.value || viewModel.network_form_dirty) {
        return
    }

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
}, { deep: true })

const isDHCP = computed(() => addressAssignment.value === "dhcp")
const isServerAddr = computed(() => props.networkAdapter?.ipv4?.addrs[0]?.addr === location.hostname)
const ipChanged = computed(() => props.networkAdapter?.ipv4?.addrs[0]?.addr !== ipAddress.value)
const dhcpChanged = computed(() => props.networkAdapter?.ipv4?.addrs[0]?.dhcp !== isDHCP.value)
const switchingToDhcp = computed(() => !props.networkAdapter?.ipv4?.addrs[0]?.dhcp && isDHCP.value)

// Modal state for rollback confirmation
const showRollbackModal = ref(false)
const enableRollback = ref(true) // Default to checked (enabled)
const isDhcpChange = ref(false) // Track if this is a DHCP change

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
		}
	}
)

const submit = async () => {
    // Check if we need to show the rollback confirmation modal
    // Show modal when:
    // 1. Static IP changed on current adapter, OR
    // 2. Switching to DHCP on current adapter (IP will likely change)
    if (isServerAddr.value && (ipChanged.value || switchingToDhcp.value)) {
        isDhcpChange.value = switchingToDhcp.value
        showRollbackModal.value = true
        return
    }

    // If not changing server IP, submit directly without rollback
    await submitNetworkConfig(false)
}

const submitNetworkConfig = async (includeRollback: boolean) => {
    isSubmitting.value = true
    showRollbackModal.value = false

    const config = JSON.stringify({
        isServerAddr: isServerAddr.value,
        ipChanged: ipChanged.value,
        name: props.networkAdapter.name,
        dhcp: isDHCP.value,
        ip: ipAddress.value ?? null,
        previousIp: props.networkAdapter.ipv4?.addrs[0]?.addr,
        netmask: netmask.value ?? null,
        gateway: gateways.value.split("\n").filter(g => g.trim()) ?? [],
        dns: dns.value.split("\n").filter(d => d.trim()) ?? [],
        enableRollback: includeRollback ? enableRollback.value : null,
        switchingToDhcp: switchingToDhcp.value
    })

    await setNetworkConfig(config)
}

const cancelRollbackModal = () => {
    showRollbackModal.value = false
}
</script>

<template>
    <div>
        <!-- Rollback Confirmation Modal -->
        <v-dialog v-model="showRollbackModal" max-width="600">
            <v-card>
                <v-card-title class="text-h5">
                    Confirm Network Configuration Change
                </v-card-title>
                <v-card-text>
                    <v-alert type="warning" variant="tonal" class="mb-4">
                        This change will disconnect your current session.
                    </v-alert>
                    <p class="mb-4">
                        <template v-if="isDhcpChange">
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
                        label="Enable automatic rollback (recommended)"
                        hide-details
                    >
                        <template #label>
                            <div>
                                <strong>Enable automatic rollback (recommended)</strong>
                                <div class="text-caption text-medium-emphasis">
                                    If you can't reach the new IP and log in within 90 seconds, the device will automatically
                                    restore the previous configuration.
                                </div>
                            </div>
                        </template>
                    </v-checkbox>
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
