<script setup lang="ts">
const props = defineProps<{
	overlay: boolean
	title: string
	text?: string
}>()
</script>

<template>
	<v-overlay :persistent="true" :model-value="props.overlay" :no-click-animation="true"
		class="align-center justify-center">
		<div id="overlay" class="flex flex-col items-center">
			<v-sheet class="flex flex-col gap-y-8 items-center p-8" :rounded="'lg'">
				<div class="text-h4 text-center">{{ props.title }}</div>
				<v-progress-circular color="secondary" indeterminate size="100" width="5"></v-progress-circular>
				<p class="text-h6 m-t-4">{{ props.text }}</p>
			</v-sheet>
		</div>
	</v-overlay>
</template>