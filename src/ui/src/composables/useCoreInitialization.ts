import { onMounted } from 'vue'
import { useCore } from './useCore'

/**
 * Composable to automatically initialize the Crux Core on component mount
 *
 * Simply call this composable in your setup() function.
 *
 * @example
 * ```ts
 * export default defineComponent({
 *   setup() {
 *     useCoreInitialization()
 *     // Rest of your setup...
 *   }
 * })
 * ```
 */
export function useCoreInitialization(): void {
  const { initialize } = useCore()

  onMounted(async () => {
    await initialize()
  })
}
