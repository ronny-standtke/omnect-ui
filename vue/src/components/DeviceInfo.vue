<script setup lang="ts">
import { type Ref, computed, onMounted, ref } from "vue"
import { useCentrifuge } from "../composables/useCentrifugo"
import { CentrifugeSubscriptionType } from "../enums/centrifuge-subscription-type.enum"
import { type FactoryReset, type FactoryResetResult, FactoryResetStatus, type OnlineStatus, type SystemInfo, type Timeouts } from "../types"
import type { UpdateValidationStatus } from "../types/update-validation-status"

const { subscribe, history, onConnected } = useCentrifuge()

const online = ref(false)
const systemInfo: Ref<SystemInfo | undefined> = ref(undefined)
const timeouts: Ref<Timeouts | undefined> = ref(undefined)
const factoryResetResult: Ref<FactoryResetResult | undefined> = ref(undefined)
const updateStatus: Ref<string> = ref("")
const factoryResetStatus = computed(() => factoryResetResult?.value?.status ?? FactoryResetStatus.Unknown)

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
			["Update status", updateStatus.value]
		])
)

const updateOnlineStatus = (data: OnlineStatus) => {
	online.value = data.iothub
}

const updateSystemInfo = (data: SystemInfo) => {
	systemInfo.value = data
}

const updateTimeouts = (data: Timeouts) => {
	timeouts.value = data
}

const updateFactoryResetStatus = (data: FactoryReset) => {
	console.log("Factory reset status update:", data)
	factoryResetResult.value = data.result
}

const updateUpdateStatus = (data: UpdateValidationStatus) => {
	updateStatus.value = data.status
}

const loadHistoryAndSubscribe = () => {
	history(updateOnlineStatus, CentrifugeSubscriptionType.OnlineStatus)
	history(updateSystemInfo, CentrifugeSubscriptionType.SystemInfo)
	history(updateTimeouts, CentrifugeSubscriptionType.Timeouts)
	history(updateFactoryResetStatus, CentrifugeSubscriptionType.FactoryReset)
	history(updateUpdateStatus, CentrifugeSubscriptionType.UpdateStatus)

	subscribe(updateOnlineStatus, CentrifugeSubscriptionType.OnlineStatus)
	subscribe(updateSystemInfo, CentrifugeSubscriptionType.SystemInfo)
	subscribe(updateTimeouts, CentrifugeSubscriptionType.Timeouts)
	subscribe(updateFactoryResetStatus, CentrifugeSubscriptionType.FactoryReset)
	subscribe(updateUpdateStatus, CentrifugeSubscriptionType.UpdateStatus)
}

onConnected(() => {
	loadHistoryAndSubscribe()
})

onMounted(() => {
	loadHistoryAndSubscribe()
})

const displayItems = computed(() => Array.from(deviceInfo.value, ([title, value]) => ({ title, value })))
</script>

<template>
  <div class="flex flex-col gap-y-8">
    <div class="text-h4 text-secondary border-b">Common Info</div>
    <dl class="grid grid-cols-[1fr_3fr] gap-x-64 gap-y-8">
      <div v-for="item of displayItems" class="">
        <dt class="font-bold text-gray-900">{{ item.title }}</dt>
        <dd class="text-gray-700 sm:col-span-2">{{ item.value }}</dd>
      </div>
      <div v-if="factoryResetStatus === FactoryResetStatus.Unknown">
        <dt class="font-bold text-gray-900">Factory Reset Status</dt>
        <dd class="text-gray-700 sm:col-span-2">n/a</dd>
      </div>
      <div v-else-if="factoryResetStatus === FactoryResetStatus.ModeSupported">
        <dt class="font-bold text-gray-900">Factory Reset Status</dt>
        <dd class="text-success sm:col-span-2">Succeeded</dd>
      </div>
      <div v-else>
        <dt class="font-bold text-gray-900">
          Factory Reset Status
          <v-tooltip :text="factoryResetResult?.paths.join(', ')">
            <template #activator="{ props }">
              <v-icon v-if="factoryResetResult?.paths.length ?? 0 > 0" icon="mdi-folder-lock-outline"
                v-bind="props"></v-icon>
            </template>
          </v-tooltip>
        </dt>
        <dd class="text-error sm:col-span-2">
          <p>
            <template v-if="factoryResetResult?.error">{{ factoryResetResult?.error }} - </template>
            {{ FactoryResetStatus[factoryResetStatus] }}
          </p>
          <p>{{ factoryResetResult?.context }}</p>
        </dd>
      </div>
    </dl>
  </div>
</template>