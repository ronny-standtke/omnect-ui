<script setup lang="ts">
import { ref, watch } from "vue"
import { useRouter } from "vue-router"
import PasswordField from "../components/common/PasswordField.vue"
import { useCore } from "../composables/useCore"
import { usePasswordForm } from "../composables/usePasswordForm"

const router = useRouter()
const { viewModel, updatePassword } = useCore()
const currentPassword = ref<string>("")
const { password, repeatPassword, errorMsg, validatePasswords } = usePasswordForm()

// flush: 'sync' fires inline during the reactive assignment, before App.vue's
// global useMessageWatchers (flush: 'pre') can clearSuccess() and erase the value.
watch(
	() => viewModel.successMessage,
	(msg) => {
		if (msg) router.push("/")
	},
	{ flush: 'sync' }
)

watch(
	() => viewModel.errorMessage,
	(msg) => {
		if (msg) errorMsg.value = msg
	},
	{ flush: 'sync' }
)

const handleSubmit = async (): Promise<void> => {
	if (!validatePasswords()) return
	await updatePassword(currentPassword.value, password.value)
}
</script>

<template>
	<v-sheet class="mx-auto pa-8 m-t-16 flex flex-col gap-y-16" border elevation="0" max-width="448" rounded="lg">
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
			<v-btn class="mb-8" color="primary" size="large" variant="flat" type="submit" block>
				Set new password
			</v-btn>
		</v-form>
	</v-sheet>
</template>
