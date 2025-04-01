<script setup lang="ts">
import { ref } from "vue"
import { useRouter } from "vue-router"
import OmnectLogo from "../components/OmnectLogo.vue"
import { useCentrifuge } from "../composables/useCentrifugo"

const { initializeCentrifuge } = useCentrifuge()
const router = useRouter()

const username = ref("")
const password = ref("")
const visible = ref(false)
const errorMsg = ref("")

const doLogin = async (e: Event) => {
	e.preventDefault()
	try {
		errorMsg.value = ""

		const creds = btoa(`${username.value}:${password.value}`)

		const res = await fetch("token/login", {
			method: "POST",
			headers: {
				Authorization: `Basic ${creds}`
			}
		})

		if (res.ok) {
			initializeCentrifuge()
			await router.push("/")
		}

		if (res.status === 401) {
			errorMsg.value = "Username and/or password wrong."
			return
		}

		errorMsg.value = "Something went wrong while logging you in."
	} catch (error) {
		errorMsg.value = "Failed to login."
	}
}
</script>

<template>
	<v-sheet class="mx-auto pa-12 pb-8 m-t-16 flex flex-col gap-y-16" border elevation="0" max-width="448" rounded="lg">
		<OmnectLogo></OmnectLogo>
		<v-form @submit.prevent @submit="doLogin">
			<v-text-field label="Username" density="compact" placeholder="Username"
				prepend-inner-icon="mdi-account-outline" variant="outlined" v-model="username" autocomplete="username"
				required></v-text-field>
			<v-text-field label="Password" :append-inner-icon="visible ? 'mdi-eye-off' : 'mdi-eye'"
				:type="visible ? 'text' : 'password'" density="compact" placeholder="Enter your password"
				prepend-inner-icon="mdi-lock-outline" variant="outlined" @click:append-inner="visible = !visible"
				v-model="password" autocomplete="current-password"></v-text-field>
			<p style="color: rgb(var(--v-theme-error))">{{ errorMsg }}</p>
			<v-btn class="mb-8" color="secondary" size="large" variant="text" type="submit" block>
				Log In
			</v-btn>
		</v-form>
	</v-sheet>
</template>