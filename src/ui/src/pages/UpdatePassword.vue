<script setup lang="ts">
import { ref } from "vue"
import { useRouter } from "vue-router"
import PasswordField from "../components/common/PasswordField.vue"
import { useCore } from "../composables/useCore"
import { useMessageWatchers } from "../composables/useMessageWatchers"
import { usePasswordForm } from "../composables/usePasswordForm"
import { useAuthNavigation } from "../composables/useAuthNavigation"

const router = useRouter()
const { updatePassword, login } = useCore()
const currentPassword = ref<string>("")
const { password, repeatPassword, errorMsg, validatePasswords } = usePasswordForm()

useAuthNavigation()

useMessageWatchers({
	onSuccess: async () => {
		// Automatically log in with the new password
		await login(password.value)
		await router.push("/")
	},
	onError: (message) => {
		errorMsg.value = message
	}
})

const handleSubmit = async (): Promise<void> => {
	if (!validatePasswords()) return
	await updatePassword(currentPassword.value, password.value)
}
</script>

<template>
	<v-sheet class="mx-auto pa-12 pb-8 m-t-16 flex flex-col gap-y-16" border elevation="0" max-width="448" rounded="lg">
		<h1>Update Password</h1>
		<v-form @submit.prevent @submit="handleSubmit">
			<PasswordField
				v-model="currentPassword"
				label="Current password"
			/>
			<PasswordField
				v-model="password"
				label="New password"
			/>
			<PasswordField
				v-model="repeatPassword"
				label="Repeat new password"
			/>
			<p style="color: rgb(var(--v-theme-error))">{{ errorMsg }}</p>
			<v-btn class="mb-8" color="secondary" size="large" variant="text" type="submit" block>
				Set new password
			</v-btn>
		</v-form>
	</v-sheet>
</template>
