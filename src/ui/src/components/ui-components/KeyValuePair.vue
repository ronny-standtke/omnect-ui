<script setup lang="ts">
const { title } = defineProps<{
	title: string
}>()
</script>

<template>
  <div>
    <dt class="font-bold text-gray-900">{{ title }}</dt>
    <dd class="text-gray-700">
      <slot></slot>
    </dd>
  </div>
</template>

<style scoped></style>