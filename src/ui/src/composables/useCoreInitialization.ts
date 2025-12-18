import { onMounted } from 'vue'
import { useCore } from './useCore'

/**
 * Composable to automatically initialize the Crux Core on component mount
 *
 * This eliminates the need to manually call initialize() in every component.
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
