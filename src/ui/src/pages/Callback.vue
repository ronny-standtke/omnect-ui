<script setup lang="ts">
import { onMounted, ref } from "vue"
import { useRouter } from "vue-router"
import { getUser, handleRedirectCallback } from "../auth/auth-service"
import OmnectLogo from "../components/branding/OmnectLogo.vue"

const router = useRouter()
const loading = ref(false)
const errorMsg = ref("")

onMounted(async () => {
	try {
		await handleRedirectCallback()
		const user = await getUser()
		if (user) {
			loading.value = true
			const res = await fetch("token/validate", {
				method: "POST",
				headers: {
					"Content-Type": "plain/text"
				},
				body: user.access_token
			})

			if (res.ok) {
				router.replace("/set-password")
			} else {
				errorMsg.value = "You are not authorized."
			}

			loading.value = false
		} else {
			errorMsg.value = "You are not authorized."
		}
	} catch (e) {
		errorMsg.value = "An error occurred while checking permissions. Please try again."
	}
})
</script>

<template>
	<v-sheet class="mx-auto pa-12 pb-8 m-t-16 flex flex-col gap-y-16 items-center" border elevation="0" max-width="448"
		rounded="lg">
		<OmnectLogo></OmnectLogo>
		<h1>Checking permissions</h1>
		<p v-if="loading">Loading...</p>
		<p class="text-error font-bold font-size-5" v-else>{{ errorMsg }}</p>
	</v-sheet>
</template>