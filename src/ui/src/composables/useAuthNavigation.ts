import { watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useCore } from './useCore'

/**
 * Composable for handling authentication-based navigation
 *
 * Automatically navigates to appropriate pages based on authentication state:
 * - Navigates to home when authenticated
 * - Navigates to set-password when password setup is required
 *
 * @example
 * ```ts
 * // In Login.vue, SetPassword.vue, or UpdatePassword.vue
 * useAuthNavigation()
 * ```
 */
export function useAuthNavigation() {
  const router = useRouter()
  const route = useRoute()
  const { viewModel } = useCore()

  // Watch for successful authentication
  watch(
    () => viewModel.isAuthenticated,
    async (isAuthenticated) => {
      if (isAuthenticated) {
        await router.push("/")
      }
    }
  )

  // Watch for requiresPasswordSet state change
  // Only navigate if we're not already on the set-password page
  watch(
    () => viewModel.requiresPasswordSet,
    async (requiresPasswordSet) => {
      if (requiresPasswordSet && route.path !== "/set-password") {
        await router.push("/set-password")
      }
    }
  )
}
