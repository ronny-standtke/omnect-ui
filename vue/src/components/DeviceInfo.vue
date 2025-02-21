<script setup lang="ts">
import { computed } from "vue"

const { deviceInfo } = defineProps<{
	deviceInfo: Map<string, string | number>
}>()

const displayItems = computed(() => Array.from(deviceInfo, ([title, value]) => ({ title, value })))
</script>

<template>
  <div class="flex flex-col gap-y-8">
    <div class="text-h4 text-secondary border-b">Common Info</div>
    <dl class="grid grid-cols-[1fr_3fr] gap-x-64 gap-y-8">
      <div v-for="item of displayItems" class="">
        <dt class="font-bold text-gray-900">{{ item.title }}</dt>
        <dd class="text-gray-700 sm:col-span-2">{{ item.value }}</dd>
      </div>
    </dl>
  </div>
</template>

<style scoped>
/* Add any specific styles for DeviceInfo.vue here */
</style>
