<script setup lang="ts">
import { type Ref, computed, onMounted, ref } from "vue"
import DeviceActions from "../components/DeviceActions.vue"
import DeviceInfo from "../components/DeviceInfo.vue"
import DeviceNetworks from "../components/DeviceNetworks.vue"
import { useCentrifuge } from "../composables/useCentrifugo"
import { useOverlaySpinner } from "../composables/useOverlaySpinner"
import { useWaitReconnect } from "../composables/useWaitReconnect"
import { CentrifugeSubscriptionType } from "../enums/centrifuge-subscription-type.enum"
import type { FactoryResetStatus, OnlineStatus, SystemInfo, Timeouts } from "../types"
import type { UpdateValidationStatus } from "../types/update-validation-status"

const { subscribe, history, onConnected } = useCentrifuge()
const { overlaySpinnerState } = useOverlaySpinner()
const { startWaitReconnect } = useWaitReconnect()

const online = ref(false)
const systemInfo: Ref<SystemInfo | undefined> = ref(undefined)
const timeouts: Ref<Timeouts | undefined> = ref(undefined)
const factoryResetStatus: Ref<string> = ref("")
const updateStatus: Ref<string> = ref("")

const deviceInfo: Ref<Map<string, string | number>> = computed(
	() =>
		new Map([
			["omnect Cloud Connection", online.value ? "connected" : "disconnected"],
			["OS name", systemInfo.value?.os.name ?? "n/a"],
			["Boot time", systemInfo.value?.boot_time ? new Date(systemInfo.value?.boot_time).toLocaleString() : "n/a"],
			["OS version", String(systemInfo.value?.os.version) ?? "n/a"],
			["Wait online timeout (in seconds)", timeouts.value?.wait_online_timeout.secs ?? "n/a"],
			["omnect device service version", systemInfo.value?.omnect_device_service_version ?? "n/a"],
			["Azure SDK version", systemInfo.value?.azure_sdk_version ?? "n/a"],
			["Factory rest status", factoryResetStatus.value],
			["Update status", updateStatus.value]
		])
)

const showIsRebooting = () => {
	overlaySpinnerState.title = "Device is rebooting"
	overlaySpinnerState.overlay = true
	startWaitReconnect()
}

const showIsResetting = () => {
	overlaySpinnerState.title = "The device is being reset"
	overlaySpinnerState.text = "Please have some patience, the resetting may take some time."
	overlaySpinnerState.overlay = true
	startWaitReconnect()
}

const updateOnlineStatus = (data: OnlineStatus) => {
	online.value = data.iothub
}

const updateSystemInfo = (data: SystemInfo) => {
	systemInfo.value = data
}

const updateTimeouts = (data: Timeouts) => {
	timeouts.value = data
}

const updateFactoryResetStatus = (data: FactoryResetStatus) => {
	factoryResetStatus.value = data.factory_reset_status
}

const updateUpdateStatus = (data: UpdateValidationStatus) => {
	updateStatus.value = data.status
}

const loadHistoryAndSubscribe = () => {
	history(updateOnlineStatus, CentrifugeSubscriptionType.OnlineStatus)
	history(updateSystemInfo, CentrifugeSubscriptionType.SystemInfo)
	history(updateTimeouts, CentrifugeSubscriptionType.Timeouts)
	history(updateFactoryResetStatus, CentrifugeSubscriptionType.FactoryResetStatus)
	history(updateUpdateStatus, CentrifugeSubscriptionType.UpdateStatus)

	subscribe(updateOnlineStatus, CentrifugeSubscriptionType.OnlineStatus)
	subscribe(updateSystemInfo, CentrifugeSubscriptionType.SystemInfo)
	subscribe(updateTimeouts, CentrifugeSubscriptionType.Timeouts)
	subscribe(updateFactoryResetStatus, CentrifugeSubscriptionType.FactoryResetStatus)
	subscribe(updateUpdateStatus, CentrifugeSubscriptionType.UpdateStatus)
}

onConnected(() => {
	loadHistoryAndSubscribe()
})

onMounted(() => {
	loadHistoryAndSubscribe()
})
</script>

<template>
	<v-sheet :border="true" rounded class="m-20">
		<div class="grid grid-cols-[1fr_auto] gap-8 gap-x-16 m-8">
			<div class="flex flex-col gap-y-16">
				<DeviceInfo :deviceInfo="deviceInfo" />
				<DeviceNetworks></DeviceNetworks>
			</div>
			<DeviceActions @reboot-in-progress="showIsRebooting" @factory-reset-in-progress="showIsResetting">
			</DeviceActions>
		</div>
	</v-sheet>
</template>

<style scoped>
.online,
.offline {
	width: 15px;
	height: 15px;
	border-radius: 15px;
}

.online {
	background-color: rgb(var(--v-theme-success));
}

.offline {
	background-color: rgb(var(--v-theme-error));
}
</style>