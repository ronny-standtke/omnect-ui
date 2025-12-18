import { ref } from 'vue'
import type { Ref } from 'vue'

/**
 * Composable for managing dialog state with auto-close on success
 *
 * @returns Object with dialog state management functions
 *
 * @example
 * ```ts
 * const { dialogs, open, close, closeAll } = useDialogState()
 *
 * // In template
 * <v-dialog v-model="dialogs.reboot">
 *
 * // In script
 * useMessageWatchers({
 *   onSuccess: closeAll
 * })
 * ```
 */
export function useDialogState<T extends string = string>() {
  const dialogs = ref<Record<string, boolean>>({}) as Ref<Record<T, boolean>>

  /**
   * Open a dialog by name
   * @param name Dialog name
   */
  const open = (name: T) => {
    dialogs.value[name] = true
  }

  /**
   * Close a dialog by name
   * @param name Dialog name
   */
  const close = (name: T) => {
    dialogs.value[name] = false
  }

  /**
   * Toggle a dialog by name
   * @param name Dialog name
   */
  const toggle = (name: T) => {
    dialogs.value[name] = !dialogs.value[name]
  }

  /**
   * Close all dialogs
   */
  const closeAll = () => {
    Object.keys(dialogs.value).forEach(key => {
      dialogs.value[key as T] = false
    })
  }

  /**
   * Check if any dialog is open
   */
  const isAnyOpen = (): boolean => {
    return Object.values(dialogs.value).some(Boolean)
  }

  return {
    dialogs,
    open,
    close,
    toggle,
    closeAll,
    isAnyOpen
  }
}
