<script setup lang="ts">
import { useFetch } from "@vueuse/core"
import { type Ref, onMounted, ref } from "vue"
import { useRouter } from "vue-router"
import { useCentrifuge } from "../composables/useCentrifugo"
import { useSnackbar } from "../composables/useSnackbar"
import { CentrifugeSubscriptionType } from "../enums/centrifuge-subscription-type.enum"
import type { NetworkStatus } from "../types"
import KeyValuePair from "./ui-components/KeyValuePair.vue"

const networkStatus: Ref<NetworkStatus | undefined> = ref(undefined)
const { snackbarState } = useSnackbar()
const router = useRouter()
const { history, subscribe } = useCentrifuge()

const showError = (errorMsg: string) => {
	snackbarState.msg = errorMsg
	snackbarState.color = "error"
	snackbarState.timeout = -1
	snackbarState.snackbar = true
}

const showSuccess = (successMsg: string) => {
	snackbarState.msg = successMsg
	snackbarState.color = "success"
	snackbarState.timeout = 2000
	snackbarState.snackbar = true
}

const {
	onFetchError: onReloadNetworkError,
	error: reloadNetworkError,
	statusCode: reloadNetworkStatusCode,
	onFetchResponse: onReloadNetworkSuccess,
	execute: reloadNetwork,
	isFetching: reloadNetworkLoading
} = useFetch("reload-network", { immediate: false }).post()

onReloadNetworkSuccess(() => {
	showSuccess("Reload network successful")
})

onReloadNetworkError(() => {
	if (reloadNetworkStatusCode.value === 401) {
		router.push("/login")
	} else {
		showError(`Reloading network failed: ${JSON.stringify(reloadNetworkError.value)}`)
	}
})

const updateNetworkStatus = (data: NetworkStatus) => {
	networkStatus.value = data
}

onMounted(() => {
	history(updateNetworkStatus, CentrifugeSubscriptionType.NetworkStatus)

	subscribe(updateNetworkStatus, CentrifugeSubscriptionType.NetworkStatus)
})
</script>

<template>
  <div class="flex flex-col gap-y-4">
    <div class="flex border-b gap-x-4 items-center">
      <div class="text-h4 text-secondary">Network</div>
      <v-btn prepend-icon="mdi-refresh" :loading="reloadNetworkLoading" @click="reloadNetwork(false)"
        variant="text">Reload
        network</v-btn>
    </div>
    <div class="gap-y-4 flex flex-col py-8" v-for="(network) of networkStatus?.network_status" :key="network.name">
      <div class="flex gap-x-4 items-center">
        <div class="text-h5">{{ network.name }}</div>
        <v-chip class="ma-2" label :color="network.online ? 'light-green-darken-2' : 'red-darken-2'">
          {{ network.online ? "online" : "offline" }}
        </v-chip>
      </div>
      <dl class=" grid grid-cols-[1fr_3fr] gap-x-64 gap-y-8">
        <KeyValuePair title="MAC address">
          {{ network.mac }}
        </KeyValuePair>
        <KeyValuePair title="IP Address">
          {{network.ipv4.addrs.map((addr) => `${addr.addr}/${addr.prefix_len} (${addr.dhcp ? "DHCP" :
            "Static"})`).join(", ")}}
        </KeyValuePair>
        <KeyValuePair title="DNS">
          {{ network.ipv4.dns.join(", ") }}
        </KeyValuePair>
        <KeyValuePair title="Gateways">
          {{ network.ipv4.gateways.join(", ") }}
        </KeyValuePair>
      </dl>
    </div>
  </div>
</template>

<style scoped></style>