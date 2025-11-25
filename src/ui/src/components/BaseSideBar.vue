<script lang="ts" setup>
import { useFetch } from "@vueuse/core"
import { onMounted, ref } from "vue"
import { useRouter } from "vue-router"
import { useDisplay } from "vuetify"

const props = defineProps<{
	showSideBar: boolean
}>()

const emits = defineEmits<(e: "drawerVisibiltyChanged", val: boolean) => void>()

const router = useRouter()
const routes = router.getRoutes()
const sidebarRoutes = ref(routes.filter((route: any) => !!route.meta.text))
const { lgAndUp } = useDisplay()

const version = ref<string>()

const { onFetchResponse: onGotResponse, execute: getVersion } = useFetch("/version", { immediate: false }).get()

onMounted(async () => {
	getVersion(false)
})

onGotResponse(async (res) => {
	version.value = await res.text()
})
</script>

<template>
	<v-navigation-drawer :modelValue="props.showSideBar"
		@update:modelValue="(val) => emits('drawerVisibiltyChanged', val)" class="bg-primary" :permanent="lgAndUp">
		<v-list class="mt-4" color="transparent" data-cy="main-nav">
			<v-list-item v-for="(route, i) in sidebarRoutes" :prepend-icon="(route.meta!.icon as string)" :key="i"
				color="white" active-class="text-white bg-white/10" :to="route.path">
				<v-list-item-title :style="{ fontWeight: 700 }"> {{ (route.meta!.text as string) }}</v-list-item-title>
			</v-list-item>
			<v-list-item active-class="text-white bg-white/10" target="_blank" rel="noopener noreferrer"
				href="https://documentation.omnect.conplement.cloud/omnect-Secure-OS/omnect-ui"
				class="text-white hover:bg-white/10">
				<v-list-item-title :style="{ fontWeight: 700 }">Documentation</v-list-item-title>
			</v-list-item>
		</v-list>
		<template v-slot:append>
			<div class="flex flex-col items-center mb-4">
				<div class="text-center w-40 text-sm lowercase">(v.{{ version }})</div>
				<a :class="`text-white hover:underline-white`" class="decoration-underline"
					href="https://www.conplement.de/en/impressum-legal-notice" target="_blank">Imprint</a>
			</div>
		</template>
	</v-navigation-drawer>
</template>