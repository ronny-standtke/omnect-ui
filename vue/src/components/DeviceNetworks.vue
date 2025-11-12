<script setup lang="ts">
import { onMounted, type Ref, ref } from "vue"
import NetworkSettings from "../components/NetworkSettings.vue"
import { useCentrifuge } from "../composables/useCentrifugo"
import { CentrifugeSubscriptionType } from "../enums/centrifuge-subscription-type.enum"
import type { NetworkStatus } from "../types"

const networkStatus: Ref<NetworkStatus | undefined> = ref(undefined)

const tab = ref(0)

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
  <div class="flex flex-col gap-y-4 flex-wrap">
    <div class="flex border-b gap-x-4 items-center">
      <div class="text-h4 text-secondary">Network</div>
    </div>
    <div class="d-flex flex-row">
      <v-tabs v-model="tab" color="primary" direction="vertical">
        <v-tab v-for="networkAdapter in networkStatus?.network_status" :text="networkAdapter.name"
          :value="networkAdapter.name"></v-tab>
      </v-tabs>
      <v-window v-model="tab" class="w[20vw]" direction="vertical">
        <v-window-item v-for="networkAdapter in networkStatus?.network_status" :value="networkAdapter.name">
          <NetworkSettings :networkAdapter="networkAdapter" />
        </v-window-item>
      </v-window>
    </div>
  </div>
</template>