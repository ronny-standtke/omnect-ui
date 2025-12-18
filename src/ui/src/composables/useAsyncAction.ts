import { ref } from 'vue'
import { useMessageWatchers } from './useMessageWatchers'

/**
 * Composable for managing async action state with automatic loading state management
 *
 * @param options Optional callbacks for success/error handling
 * @returns Object with loading state and execute function
 *
 * @example
 * ```ts
 * const { loading, execute } = useAsyncAction({
 *   onSuccess: () => {
 *     dialog.value = false
 *   }
 * })
 *
 * const handleSubmit = () => execute(async () => {
 *   await someAction()
 * })
 * ```
 */
export function useAsyncAction(options?: {
  onSuccess?: (message: string) => void
  onError?: (message: string) => void
}) {
  const loading = ref(false)

  useMessageWatchers({
    onSuccess: (message) => {
      loading.value = false
      options?.onSuccess?.(message)
    },
    onError: (message) => {
      loading.value = false
      options?.onError?.(message)
    }
  })

  /**
   * Execute an async action with automatic loading state management
   * @param action The async function to execute
   */
  const execute = async (action: () => Promise<void>) => {
    loading.value = true
    await action()
  }

  return { loading, execute }
}
