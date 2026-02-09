<script setup lang="ts">
import axios from "axios"
import { onMounted, type Ref, ref, computed, watch } from "vue"
import { useRoute, useRouter } from "vue-router"
import { useDisplay } from "vuetify"
import BaseSideBar from "./components/BaseSideBar.vue"
import DialogContent from "./components/DialogContent.vue"
import OmnectLogo from "./components/branding/OmnectLogo.vue"
import OverlaySpinner from "./components/feedback/OverlaySpinner.vue"
import UserMenu from "./components/UserMenu.vue"
import { useCore } from "./composables/useCore"
import type { HealthcheckInfo } from "./composables/useCore"
import { useSnackbar } from "./composables/useSnackbar"
import { useMessageWatchers } from "./composables/useMessageWatchers"

axios.defaults.validateStatus = (_) => true

const { snackbarState } = useSnackbar()
const { viewModel, ackRollback, ackFactoryResetResult, ackUpdateValidation, subscribeToChannels, unsubscribeFromChannels } = useCore()

// Enable automatic message watchers — suppress error toasts on pages that show errors inline
useMessageWatchers({
	suppressErrorToast: () => route.meta.inlineErrors === true
})

const { lgAndUp } = useDisplay()
const router = useRouter()
const route = useRoute()
const showSideBar: Ref<boolean> = ref(lgAndUp.value)
const overlay: Ref<boolean> = ref(false)
const showRollbackNotification: Ref<boolean> = ref(false)
const showFactoryResetResultModal: Ref<boolean> = ref(false)
const showUpdateValidationModal: Ref<boolean> = ref(false)
const factoryResetAckedOnMount = ref(false)
const updateValidationAckedOnMount = ref(false)
const errorTitle = ref("")
const errorMsg = ref("")

const overlaySpinnerState = computed(() => viewModel.overlaySpinner)

// Build redirect URL for overlay spinner button
const redirectUrl = computed(() => {
	const networkState = viewModel.networkChangeState

	// Network change: show button for waiting and timeout states (but NOT for DHCP or rollback verification)
	if ((networkState.type === 'waitingForNewIp' || networkState.type === 'newIpTimeout')
		&& 'newIp' in networkState
		&& 'uiPort' in networkState
		&& !('switchingToDhcp' in networkState && networkState.switchingToDhcp)) {
		return `https://${networkState.newIp}:${networkState.uiPort}`
	}

	// Device operations: show button on timeout (same address, for cert re-acceptance)
	if (viewModel.deviceOperationState.type === 'reconnectionFailed') {
		return window.location.href
	}

	return undefined
})

// Countdown label depends on context
const countdownLabel = computed(() => {
	const networkState = viewModel.networkChangeState
	if (networkState.type !== 'idle') {
		return 'Automatic rollback in:'
	}
	const deviceState = viewModel.deviceOperationState
	if (deviceState.type === 'rebooting' || deviceState.type === 'factoryResetting'
		|| deviceState.type === 'updating' || deviceState.type === 'waitingReconnection') {
		return 'Timeout in:'
	}
	return undefined
})

const toggleSideBar = () => {
	showSideBar.value = !showSideBar.value
}

const updateSidebarVisibility = (visible: boolean) => {
	showSideBar.value = visible
}

const acknowledgeRollback = () => {
	ackRollback()
	showRollbackNotification.value = false
}

const acknowledgeFactoryResetResult = () => {
	ackFactoryResetResult()
	showFactoryResetResultModal.value = false
}

const acknowledgeUpdateValidation = () => {
	ackUpdateValidation()
	showUpdateValidationModal.value = false
}

const factoryResetIsSuccess = computed(() =>
	viewModel.factoryReset?.result?.status === 'modeSupported'
)

// Watch authentication state to redirect to login if session is lost
// This handles the case where the backend restarts (reboot/factory reset) and the session becomes invalid
watch(
	() => viewModel.isAuthenticated,
	async (isAuthenticated) => {
		if (isAuthenticated) {
			subscribeToChannels()
		} else {
			unsubscribeFromChannels()
			if (route.meta.requiresAuth) {
				await router.push("/login")
			}
		}
	},
    { immediate: true }
)

// Watch for network rollback status from healthcheck updates (e.g. after automatic rollback)
watch(
	() => viewModel.healthcheck?.networkRollbackOccurred,
	(occurred) => {
		if (occurred) {
			showRollbackNotification.value = true
		}
	}
)

// Watch for factory reset result (arrives via WebSocket after republish)
watch(
	() => viewModel.factoryReset?.result,
	(result) => {
		if (result && result.status !== 'unknown' && !factoryResetAckedOnMount.value) {
			showFactoryResetResultModal.value = true
		}
	}
)

// Watch for update validation status (arrives via WebSocket and healthcheck)
watch(
	() => viewModel.updateValidationStatus?.status,
	(status) => {
		if ((status === 'Succeeded' || status === 'Recovered') && !updateValidationAckedOnMount.value) {
			showUpdateValidationModal.value = true
		}
	}
)

onMounted(async () => {
	const res = await fetch("healthcheck", {
		headers: {
			"Cache-Control": "no-cache, no-store, must-revalidate",
			Pragma: "no-cache",
			Expires: "0"
		}
	})
	const data = (await res.json()) as HealthcheckInfo
	if (data.networkRollbackOccurred) {
		showRollbackNotification.value = true
	}

	// Record acked state to suppress watcher-triggered modals for already-acked results
	factoryResetAckedOnMount.value = (data as any).factoryResetResultAcked ?? false
	updateValidationAckedOnMount.value = (data as any).updateValidationAcked ?? false

	// Check update validation on mount (factory reset result arrives via WebSocket)
	if (!updateValidationAckedOnMount.value) {
		const status = data.updateValidationStatus?.status
		if (status === 'Succeeded' || status === 'Recovered') {
			showUpdateValidationModal.value = true
		}
	}

	if (!res.ok) {
		overlay.value = true
		errorTitle.value = "omnect-device-service version mismatch"
		errorMsg.value = `Current version: ${data.versionInfo.current}. Required version ${data.versionInfo.required}. Please consider to update omnect Secure OS.`
	}
})
</script>

<template>
  <v-app>
    <v-dialog v-model="overlay" max-width="50vw" :no-click-animation="true" persistent fullscreen>
      <DialogContent :title="errorTitle" dialog-type="Error" :show-close="false">
        <div class="flex flex-col gap-2 mb-8">
          {{ errorMsg }}
        </div>
      </DialogContent>
    </v-dialog>
    <v-dialog v-model="showRollbackNotification" max-width="500" persistent>
      <DialogContent title="Network Settings Rolled Back" dialog-type="Warning" :show-close="false">
        <div class="flex flex-col gap-4 mb-4">
          <p>
            The network settings were rolled back to the previous configuration because the new settings could not be confirmed.
          </p>
          <div class="flex justify-end">
            <v-btn color="primary" @click="acknowledgeRollback">OK</v-btn>
          </div>
        </div>
      </DialogContent>
    </v-dialog>
    <v-dialog v-model="showFactoryResetResultModal" max-width="500" persistent>
      <DialogContent
        :title="factoryResetIsSuccess ? 'Factory Reset Completed' : 'Factory Reset Failed'"
        :dialog-type="factoryResetIsSuccess ? 'Success' : 'Error'"
        :show-close="false">
        <div class="flex flex-col gap-4 mb-4">
          <template v-if="factoryResetIsSuccess">
            <p>The factory reset completed successfully.</p>
          </template>
          <template v-else>
            <p v-if="viewModel.factoryReset?.result?.error">
              {{ viewModel.factoryReset.result.error }}
            </p>
            <p v-if="viewModel.factoryReset?.result?.context">
              {{ viewModel.factoryReset.result.context }}
            </p>
          </template>
          <div class="flex justify-end">
            <v-btn color="primary" @click="acknowledgeFactoryResetResult">OK</v-btn>
          </div>
        </div>
      </DialogContent>
    </v-dialog>
    <v-dialog v-model="showUpdateValidationModal" max-width="500" persistent>
      <DialogContent
        :title="viewModel.updateValidationStatus?.status === 'Recovered'
          ? 'Update Rolled Back'
          : 'Update Succeeded'"
        :dialog-type="viewModel.updateValidationStatus?.status === 'Recovered'
          ? 'Warning'
          : 'Success'"
        :show-close="false">
        <div class="flex flex-col gap-4 mb-4">
          <template v-if="viewModel.updateValidationStatus?.status === 'Succeeded'">
            <p>The firmware update was applied and validated successfully.</p>
          </template>
          <template v-else>
            <p>The firmware update could not be validated. The previous version has been restored.</p>
          </template>
          <div class="flex justify-end">
            <v-btn color="primary" @click="acknowledgeUpdateValidation">OK</v-btn>
          </div>
        </div>
      </DialogContent>
    </v-dialog>
    <v-app-bar flat :style="{ borderBottomWidth: '1px', borderColor: '#677680' }">
      <template #prepend>
        <v-icon class="hidden-lg-and-up mr-4 cursor-pointer text-primary" @click.stop="toggleSideBar">mdi-menu</v-icon>
        <OmnectLogo class="h-12"></OmnectLogo>
      </template>
      <template v-if="route.meta.showMenu" #append>
        <div class="flex gap-x-4 mr-4 items-center">
          <UserMenu />
        </div>
      </template>
    </v-app-bar>
    <BaseSideBar v-if="route.meta.showMenu" :showSideBar="showSideBar"
      @drawerVisibiltyChanged="updateSidebarVisibility">
    </BaseSideBar>
    <v-main>
      <RouterView></RouterView>
      <v-snackbar v-model="snackbarState.snackbar" :color="snackbarState.color" :timeout="snackbarState.timeout">
        {{ snackbarState.msg }}
        <template #actions>
          <v-btn icon=" mdi-close" @click="snackbarState.snackbar = false"></v-btn>
        </template>
      </v-snackbar>
      <OverlaySpinner :overlay="overlaySpinnerState.overlay" :title="overlaySpinnerState.title"
        :text="overlaySpinnerState.text || undefined" :timed-out="overlaySpinnerState.timedOut"
        :progress="overlaySpinnerState.progress || undefined"
        :countdown-seconds="overlaySpinnerState.countdownSeconds || undefined"
        :countdown-label="countdownLabel"
        :redirect-url="redirectUrl" />
    </v-main>
  </v-app>
</template>

<style>
:root {
  --color-primary: #677680;
  --color-background: #f4f5f7;
  --color-header: #dee2e6;
  --color-primary-rgb: 103, 118, 128;
  --color-secondary: #0094b1;
  --color-grey-30: #afb1b3;
  --color-grey-10: #e1e4e6;
  --color-grey-5: #f2f2f2;
  --color-white-dimmed: #d9d9d9;
  --color-notification-success-fill: #6ca425;
  --color-notification-setup-fill: #de5c14;
  --color-notification-setup-text: #f2d3c2;
  --color-notification-update-fill: #008a96;
  --color-notification-update-text: #c2eef2;
  --color-notification-general-fill: #388bc7;
  --color-notification-common-fill: #005d86;
  --color-notification-general-text: #c2e4f2;
  --color-notification-cancelled-fill: #ffb100;
  --color-notification-pending-fill: #bd5ec0;
  --color-text-primary: #292f33;
  --color-fail: #b3101d;
  font-size: 16px;
}

body {
  color: var(--color-text-primary);
  border-color: var(--color-grey-30);
}

p,
ul,
ol,
pre {
  margin: '1em 0';
  line-height: 1.75;
}

blockquote {
  margin: '1em 0';
  padding-left: '1em';
  font-style: 'italic';
  border-left: '.25em solid var(--un-prose-borders)';
}

img,
video {
  max-width: 100%;
}

figure,
picture {
  margin: 1em 0;
}

figcaption {
  color: var(--un-prose-captions);
  font-size: .875em;
}

table {
  margin: 1em 0;
  border-collapse: collapse;
  overflow-x: auto;
}

td,
th {
  padding: .625em 1em;
}

th {
  font-weight: 600;
}

abbr {
  cursor: help;
}

kbd {
  color: var(--un-prose-code);
  border: 1px solid;
  padding: .25rem .5rem;
  font-size: .875em;
  border-radius: .25rem;
}

details {
  margin: 1em 0;
  padding: 1.25rem 1.5rem;
  background: var(--un-prose-bg-soft);
}

summary {
  cursor: pointer;
  font-weight: 600;
}

.v-data-table-header__content {
  font-weight: 600;
}

.v-table__wrapper {
  overflow: visible !important;
}

.white_30 {
  background-color: rgb(255 255 255 / 0.3)
}
</style>
