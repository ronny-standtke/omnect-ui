<script setup lang="ts">
import { watch } from "vue"
import OmnectLogo from "../components/branding/OmnectLogo.vue"
import PasswordField from "../components/common/PasswordField.vue"
import { useCore } from "../composables/useCore"
import { useCoreInitialization } from "../composables/useCoreInitialization"
import { usePasswordForm } from "../composables/usePasswordForm"
import { useAuthNavigation } from "../composables/useAuthNavigation"

const { viewModel, setPassword } = useCore()
const { password, repeatPassword, errorMsg, validatePasswords } = usePasswordForm()

useCoreInitialization()
// SetPasswordResponse now directly authenticates (sets auth_token + is_authenticated),
// so useAuthNavigation's isAuthenticated watcher handles the redirect to home.
useAuthNavigation()

watch(
	() => viewModel.errorMessage,
	(msg) => {
		if (msg) errorMsg.value = msg
	},
	{ flush: 'sync' }
)

const handleSubmit = async (): Promise<void> => {
	if (!validatePasswords()) return
	await setPassword(password.value)
}
</script>

<template>
	<v-sheet class="mx-auto pa-8 m-t-16 flex flex-col gap-y-16" border elevation="0" max-width="448" rounded="lg">
		<OmnectLogo></OmnectLogo>
		<h1>Set Password</h1>
		<v-form @submit.prevent @submit="handleSubmit">
			<PasswordField
				v-model="password"
				label="Password"
			/>
			<PasswordField
				v-model="repeatPassword"
				label="Repeat password"
			/>
			<p style="color: rgb(var(--v-theme-error))">{{ errorMsg }}</p>
			<v-btn class="mb-8" color="primary" size="large" variant="flat" type="submit" block>
				Set password
			</v-btn>
		</v-form>
	</v-sheet>
</template>