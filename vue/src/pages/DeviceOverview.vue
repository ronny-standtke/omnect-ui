<script setup lang="ts">
import { type Ref, computed, onMounted, ref } from "vue"
import DeviceActions from "../components/DeviceActions.vue"
import DeviceInfo from "../components/DeviceInfo.vue"
import DeviceNetworks from "../components/DeviceNetworks.vue"
import { useCentrifuge } from "../composables/useCentrifugo"
import { CentrifugeSubscriptionType } from "../enums/centrifuge-subscription-type.enum"
import type { FactoryResetStatus, OnlineStatus, SystemInfo, Timeouts } from "../types"
const { subscribe, history, onConnected } = useCentrifuge()

const online = ref(false)
const systemInfo: Ref<SystemInfo | undefined> = ref(undefined)
const timeouts: Ref<Timeouts | undefined> = ref(undefined)
const factoryResetStatus: Ref<string> = ref("")
const isResetting = ref(false)
const isRebooting = ref(false)

const deviceInfo: Ref<Map<string, string | number>> = computed(
	() =>
		new Map([
			["Online", String(online.value)],
			["OS name", systemInfo.value?.os.name ?? "n/a"],
			["Boot time", systemInfo.value?.boot_time ? new Date(systemInfo.value?.boot_time).toLocaleString() : "n/a"],
			["OS version", String(systemInfo.value?.os.version) ?? "n/a"],
			["Wait online timeout (in seconds)", timeouts.value?.wait_online_timeout.secs ?? "n/a"],
			["omnect device service version", systemInfo.value?.omnect_device_service_version ?? "n/a"],
			["Azure SDK version", systemInfo.value?.azure_sdk_version ?? "n/a"],
			["Factory rest status", factoryResetStatus.value]
		])
)

onConnected(() => {
	isResetting.value = false
	isRebooting.value = false
})

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

onMounted(() => {
	history(updateOnlineStatus, CentrifugeSubscriptionType.OnlineStatus)
	history(updateSystemInfo, CentrifugeSubscriptionType.SystemInfo)
	history(updateTimeouts, CentrifugeSubscriptionType.Timeouts)
	history(updateFactoryResetStatus, CentrifugeSubscriptionType.FactoryResetStatus)

	subscribe(updateOnlineStatus, CentrifugeSubscriptionType.OnlineStatus)
	subscribe(updateSystemInfo, CentrifugeSubscriptionType.SystemInfo)
	subscribe(updateTimeouts, CentrifugeSubscriptionType.Timeouts)
	subscribe(updateFactoryResetStatus, CentrifugeSubscriptionType.FactoryResetStatus)
})
</script>

<template>
	<v-overlay :persistent="true" :model-value="isResetting || isRebooting" z-index="1000"
		class="align-center justify-center">
		<div id="overlay" class="flex flex-col items-center">
			<v-sheet class="flex flex-col gap-y-8 items-center p-8" :rounded="'lg'">
				<div v-if="isRebooting" class="text-h4 text-center">Device is rebooting</div>
				<div v-else-if="isResetting" class="text-h4 text-center">Device is resetting</div>
				<v-progress-circular color="secondary" indeterminate size="100" width="5"></v-progress-circular>
				<p v-if="isResetting" class="text-h6 m-t-4">Please have some patience, the resetting may take some time.
				</p>
			</v-sheet>
		</div>
	</v-overlay>

	<v-sheet :border="true" rounded class="m-20">
		<div class="grid grid-cols-[1fr_auto] gap-8 gap-x-16 m-8">
			<div class="flex flex-col gap-y-16">
				<DeviceInfo :deviceInfo="deviceInfo" />
				<DeviceNetworks></DeviceNetworks>
			</div>
			<DeviceActions @reboot-in-progress="isRebooting = true" @factory-reset-in-progress="isResetting = true">
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