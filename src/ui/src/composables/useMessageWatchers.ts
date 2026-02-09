import { watch } from 'vue'
import { useSnackbar } from './useSnackbar'
import { useCore } from './useCore'

/**
 * Composable to automatically watch and display success/error messages from the Core
 *
 * @param options Optional callbacks to execute when messages are received
 * @returns void
 *
 * @example
 * ```ts
 * // Basic usage - just show messages
 * useMessageWatchers()
 *
 * // With callbacks
 * useMessageWatchers({
 *   onSuccess: () => {
 *     loading.value = false
 *     dialog.value = false
 *   },
 *   onError: () => {
 *     loading.value = false
 *   }
 * })
 * ```
 */
export function useMessageWatchers(options?: {
  onSuccess?: (message: string) => void
  onError?: (message: string) => void
  suppressErrorToast?: () => boolean
}): void {
  const { viewModel, clearSuccess, clearError } = useCore()
  const { showSuccess, showError } = useSnackbar()

  // Watch for success messages
  watch(
    () => viewModel.successMessage,
    (newMessage) => {
      if (newMessage) {
        showSuccess(newMessage)
        options?.onSuccess?.(newMessage)
        clearSuccess()
      }
    }
  )

  // Watch for error messages
  watch(
    () => viewModel.errorMessage,
    (newMessage) => {
      if (newMessage) {
        if (!options?.suppressErrorToast?.()) {
          showError(newMessage)
        }
        options?.onError?.(newMessage)
        clearError()
      }
    }
  )
}
