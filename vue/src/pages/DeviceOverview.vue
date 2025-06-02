<script setup lang="ts">
import DeviceActions from "../components/DeviceActions.vue"
import DeviceInfo from "../components/DeviceInfo.vue"
import DeviceNetworks from "../components/DeviceNetworks.vue"
import { useOverlaySpinner } from "../composables/useOverlaySpinner"
import { useWaitReconnect } from "../composables/useWaitReconnect"

const { overlaySpinnerState } = useOverlaySpinner()
const { startWaitReconnect } = useWaitReconnect()

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
</script>

<template>
	<v-sheet :border="true" rounded class="m-20">
		<div class="grid grid-cols-[1fr_auto] gap-8 gap-x-16 m-8">
			<div class="flex flex-col gap-y-16">
				<DeviceInfo />
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