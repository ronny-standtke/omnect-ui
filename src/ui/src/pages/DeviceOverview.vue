<script setup lang="ts">
import DeviceActions from "../components/device/DeviceActions.vue"
import DeviceInfo from "../components/device/DeviceInfoCore.vue"
</script>

<template>
	<v-sheet :border="true" rounded class="ma-4">
		<div class="grid grid-cols-[1fr_auto] gap-8 gap-x-16 ma-4">
			<div class="flex flex-col gap-y-16">
				<DeviceInfo />
			</div>
			<DeviceActions></DeviceActions>
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