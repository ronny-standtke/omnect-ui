import { ref } from 'vue'

/**
 * Composable for managing password form state with validation
 *
 * @returns Object with password refs, error state, and validation
 *
 * @example
 * ```ts
 * const { password, repeatPassword, errorMsg, validatePasswords } = usePasswordForm()
 *
 * const handleSubmit = async () => {
 *   if (!validatePasswords()) return
 *   await setPassword(password.value)
 * }
 * ```
 */
export function usePasswordForm() {
  const password = ref<string>("")
  const repeatPassword = ref<string>("")
  const errorMsg = ref("")

  /**
   * Validate that passwords match
   * @returns true if passwords match, false otherwise
   */
  const validatePasswords = (): boolean => {
    errorMsg.value = ""
    if (password.value !== repeatPassword.value) {
      errorMsg.value = "Passwords do not match."
      return false
    }
    return true
  }

  /**
   * Clear all form state
   */
  const clearForm = () => {
    password.value = ""
    repeatPassword.value = ""
    errorMsg.value = ""
  }

  return {
    password,
    repeatPassword,
    errorMsg,
    validatePasswords,
    clearForm
  }
}
