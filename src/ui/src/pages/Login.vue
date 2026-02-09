<script setup lang="ts">
import { onMounted, ref, watch } from "vue"
import OmnectLogo from "../components/branding/OmnectLogo.vue"
import { useCore } from "../composables/useCore"
import { useAuthNavigation } from "../composables/useAuthNavigation"

const { viewModel, login, checkRequiresPasswordSet, initialize } = useCore()

const password = ref("")
const visible = ref(false)
const isCheckingPasswordSetNeeded = ref(false)
const errorMsg = ref("")

useAuthNavigation()

watch(
	() => viewModel.errorMessage,
	(msg) => {
		if (msg) errorMsg.value = msg
	},
	{ flush: 'sync' }
)

const doLogin = async (e: Event) => {
	e.preventDefault()
	errorMsg.value = ""
	await login(password.value)
}

onMounted(async () => {
	isCheckingPasswordSetNeeded.value = true
	// Initialize Core first, then check if password needs to be set
	await initialize()
	await checkRequiresPasswordSet()
	isCheckingPasswordSetNeeded.value = false
})
</script>

<template>
	<v-sheet class="mx-auto pa-8 m-t-16 flex flex-col gap-y-16" border elevation="0" max-width="448" rounded="lg">
		<OmnectLogo></OmnectLogo>
		<v-form v-if="!isCheckingPasswordSetNeeded" @submit.prevent @submit="doLogin">
			<v-text-field label="Password" :append-inner-icon="visible ? 'mdi-eye-off' : 'mdi-eye'"
				:type="visible ? 'text' : 'password'" density="compact" placeholder="Enter your password"
				prepend-inner-icon="mdi-lock-outline" variant="outlined" @click:append-inner="visible = !visible"
				v-model="password" autocomplete="current-password"></v-text-field>
			<p style="color: rgb(var(--v-theme-error))">{{ errorMsg }}</p>
			<v-btn class="mb-8" color="primary" size="large" variant="flat" type="submit" block>
				Log In
			</v-btn>
		</v-form>
	</v-sheet>
</template>