<script setup lang="ts">
import { computed, ref } from "vue"
import { useSnackbar } from "../composables/useSnackbar"
import type { DeviceNetwork } from "../types"
import { useWaitForNewIp } from "../composables/useWaitForNewIp";
import { useOverlaySpinner } from "../composables/useOverlaySpinner";

const { showSuccess, showError } = useSnackbar()
const { overlaySpinnerState } = useOverlaySpinner()
const { startWaitForNewIp, onConnected } = useWaitForNewIp()

const props = defineProps<{
    networkAdapter: DeviceNetwork
}>()

const ipAddress = ref(props.networkAdapter?.ipv4?.addrs[0]?.addr || "")
const dns = ref(props.networkAdapter?.ipv4?.dns?.join("\n") || "")
const gateways = ref(props.networkAdapter?.ipv4?.gateways?.join("\n") || "")
const addressAssignment = ref(props.networkAdapter?.ipv4?.addrs[0]?.dhcp ? "dhcp" : "static")
const netmask = ref(props.networkAdapter?.ipv4?.addrs[0]?.prefix_len || 24)
const isDHCP = computed(() => addressAssignment.value === "dhcp")
const isSubmitting = ref(false)
const isServerAddr = computed(() => props.networkAdapter?.ipv4?.addrs[0]?.addr === location.hostname)
const ipChanged = computed(() => props.networkAdapter?.ipv4?.addrs[0]?.addr !== ipAddress.value)

const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text).then(() => {
        showSuccess("Copied to clipboard")
    })
}

const restoreSettings = () => {
    ipAddress.value = props.networkAdapter?.ipv4?.addrs[0]?.addr || ""
    addressAssignment.value = props.networkAdapter?.ipv4?.addrs[0]?.dhcp ? "dhcp" : "static"
    netmask.value = props.networkAdapter?.ipv4?.addrs[0]?.prefix_len || 24
    dns.value = props.networkAdapter?.ipv4?.dns?.join("\n") || ""
    gateways.value = props.networkAdapter?.ipv4?.gateways?.join("\n") || ""
}

const isValidIp = (value: string) => {
    if (!value) return true
    const regex = /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}$/
    return regex.test(value) || "Invalid IPv4-Address"
}

const setNetMask = (mask: string) => {
    const prefixLen = Number.parseInt(mask.replace("/", ""), 10)
    if (isNaN(prefixLen) || prefixLen < 0 || prefixLen > 32) {
        return "Invalid netmask"
    }
    netmask.value = prefixLen
}

onConnected(() => {
    window.location.replace(`https://${ipAddress.value}:${window.location.port}`)
})

const submit = async () => {
    try {
        isSubmitting.value = true

        const res = await fetch("network", {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify({
                isServerAddr: isServerAddr.value,
                ipChanged: ipChanged.value,
                name: props.networkAdapter.name,
                dhcp: isDHCP.value,
                ip: ipAddress.value ?? null,
                previousIp: props.networkAdapter.ipv4?.addrs[0]?.addr,
                netmask: netmask.value ?? null,
                gateway: gateways.value.split("\n") ?? [],
                dns: dns.value.split("\n") ?? []
            })
        })

        if (res.ok) {
            if (isServerAddr.value && ipChanged.value) {
                overlaySpinnerState.title = "Applying network setting"
                overlaySpinnerState.text = "The network settings are applied. You will be forwarded to the new IP. Log in to confirm the settings.If you do not log in within 90 seconds, the IP will be reset."
                overlaySpinnerState.overlay = true
                startWaitForNewIp(`https://${ipAddress.value}:${window.location.port}`)
            } else {
                showSuccess("Network setting set successfully")
            }
        } else {
            const errorMsg = await res.text()
            showError(`Failed to set network settings: ${errorMsg}`)
        }
    } catch (e) {
        showError(`Failed to set network settings: ${e}`)
    } finally {
        isSubmitting.value = false
    }
}
</script>

<template>
    <div>
        <v-form @submit.prevent="submit" class="flex flex-col gap-y-4 ml-4">
            <v-chip size="large" class="ma-2" label
                :color="props.networkAdapter.online ? 'light-green-darken-2' : 'red-darken-2'">
                {{ props.networkAdapter.online ? "Online" : "Offline" }}
            </v-chip>
            <v-radio-group v-model="addressAssignment" inline>
                <v-radio label="DHCP" value="dhcp"></v-radio>
                <v-radio label="Static" value="static"></v-radio>
            </v-radio-group>
            <v-text-field :readonly="isDHCP" v-model="ipAddress" label="IP Address" :rules="[isValidIp]" outlined
                append-inner-icon="mdi-content-copy" @click:append-inner="copyToClipboard(`${ipAddress}/${netmask}`)">
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
                @click:append-inner="copyToClipboard(props.networkAdapter.mac)"></v-text-field>
            <v-textarea :readonly="isDHCP" v-model="gateways" label="Gateways" variant="outlined" rows="3" no-resize
                append-inner-icon="mdi-content-copy" @click:append-inner="copyToClipboard(ipAddress)"></v-textarea>
            <v-textarea v-model="dns" label="DNS" variant="outlined" rows="3" no-resize
                append-inner-icon="mdi-content-copy" @click:append-inner="copyToClipboard(ipAddress)"></v-textarea>
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
