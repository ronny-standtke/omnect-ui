<script setup lang="ts">
import { onMounted, type Ref, ref } from "vue"
import DeviceNetworks from "../components/DeviceNetworks.vue"
import NetworkActions from "../components/NetworkActions.vue"
import { useCentrifuge } from "../composables/useCentrifugo"
import { CentrifugeSubscriptionType } from "../enums/centrifuge-subscription-type.enum"
import type { NetworkStatus } from "../types"

const networkStatus: Ref<NetworkStatus | undefined> = ref(undefined)

const { history, subscribe, onConnected } = useCentrifuge()

const updateNetworkStatus = (data: NetworkStatus) => {
	networkStatus.value = data
}

const loadHistoryAndSubscribe = () => {
	history(updateNetworkStatus, CentrifugeSubscriptionType.NetworkStatus)
	subscribe(updateNetworkStatus, CentrifugeSubscriptionType.NetworkStatus)
}

onConnected(() => {
	loadHistoryAndSubscribe()
})

onMounted(() => {
	loadHistoryAndSubscribe()
})
</script>

<template>
    <v-sheet :border="true" rounded class="m-20">
        <div class="grid grid-cols-[1fr_minmax(200px,auto)] gap-8 gap-x-16 m-8">
            <div class="flex flex-col gap-y-16">
                <DeviceNetworks></DeviceNetworks>
            </div>
            <NetworkActions></NetworkActions>
        </div>
    </v-sheet>
</template>