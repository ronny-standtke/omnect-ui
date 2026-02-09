<script setup lang="ts">
/**
 * DeviceInfoCore - Device information component using Crux Core state management
 *
 * This component uses the Crux architecture where:
 * - All state lives in the Core
 * - Shell handles only effects (HTTP, Centrifugo)
 * - Components read from the reactive viewModel
 * - No local refs for data - all computed from Core state
 */
import { computed } from 'vue'
import { useCore } from '../../composables/useCore'
import { useCoreInitialization } from '../../composables/useCoreInitialization'

const { viewModel } = useCore()

useCoreInitialization()

// All device info computed from the Core's viewModel
const deviceInfo = computed(
  () =>
    new Map([
      ['omnect Cloud Connection', viewModel.onlineStatus?.iothub ? 'connected' : 'disconnected'],
      ['omnect Secure OS variant', viewModel.systemInfo?.os.name ?? 'n/a'],
      [
        'Boot time',
        viewModel.systemInfo?.bootTime
          ? new Date(viewModel.systemInfo.bootTime).toLocaleString()
          : 'n/a',
      ],
      ['omnect Secure OS version', String(viewModel.systemInfo?.os.version) ?? 'n/a'],
      ['Wait online timeout (in seconds)', viewModel.timeouts?.waitOnlineTimeout.secs ?? 'n/a'],
      [
        'omnect device service version',
        viewModel.systemInfo?.omnectDeviceServiceVersion ?? 'n/a',
      ],
      ['Azure SDK version', viewModel.systemInfo?.azureSdkVersion ?? 'n/a'],
      ['Update status', viewModel.updateValidationStatus?.status ?? 'n/a'],
    ])
)

// Factory reset status from Core
const factoryResetStatus = computed(() => viewModel.factoryReset?.result?.status ?? 'unknown')
const factoryResetResult = computed(() => viewModel.factoryReset?.result ?? null)

// Map Core status strings to display values
const factoryResetDisplayStatus = computed(() => {
  switch (factoryResetStatus.value) {
    case 'modeSupported':
      return 'Succeeded'
    case 'modeUnsupported':
      return 'Mode Unsupported'
    case 'backupRestoreError':
      return 'Backup/Restore Error'
    case 'configurationError':
      return 'Configuration Error'
    default:
      return 'n/a'
  }
})

const isSuccess = computed(() => factoryResetStatus.value === 'modeSupported')
const isError = computed(
  () =>
    factoryResetStatus.value !== 'unknown' && factoryResetStatus.value !== 'modeSupported'
)

const displayItems = computed(() =>
  Array.from(deviceInfo.value, ([title, value]) => ({ title, value }))
)
</script>

<template>
  <div class="flex flex-col gap-y-8">
    <div class="text-h4 text-secondary border-b">Common Info</div>
    <dl class="grid grid-cols-[1fr_3fr] gap-x-64 gap-y-8">
      <div v-for="item of displayItems" :key="item.title">
        <dt class="font-bold text-gray-900">{{ item.title }}</dt>
        <dd class="text-gray-700 sm:col-span-2">{{ item.value }}</dd>
      </div>
      <div v-if="factoryResetStatus === 'unknown'">
        <dt class="font-bold text-gray-900">Factory Reset Status</dt>
        <dd class="text-gray-700 sm:col-span-2">n/a</dd>
      </div>
      <div v-else-if="isSuccess">
        <dt class="font-bold text-gray-900">Factory Reset Status</dt>
        <dd class="text-success sm:col-span-2">Succeeded</dd>
      </div>
      <div v-else-if="isError">
        <dt class="font-bold text-gray-900">
          Factory Reset Status
          <v-tooltip :text="factoryResetResult?.paths.join(', ')">
            <template #activator="{ props }">
              <v-icon
                v-if="(factoryResetResult?.paths.length ?? 0) > 0"
                icon="mdi-folder-lock-outline"
                v-bind="props"
              ></v-icon>
            </template>
          </v-tooltip>
        </dt>
        <dd class="text-error sm:col-span-2">
          <p>
            <template v-if="factoryResetResult?.error"
              >{{ factoryResetResult?.error }} - </template
            >
            {{ factoryResetDisplayStatus }}
          </p>
          <p>{{ factoryResetResult?.context }}</p>
        </dd>
      </div>
    </dl>
  </div>
</template>
