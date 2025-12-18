import { useSnackbar } from './useSnackbar'

/**
 * Composable for copying text to clipboard with user feedback
 *
 * @returns Object with copy function
 *
 * @example
 * ```ts
 * const { copy } = useClipboard()
 *
 * // Basic usage
 * copy('text to copy')
 *
 * // Custom success message
 * copy('192.168.1.1', 'IP address copied')
 * ```
 */
export function useClipboard() {
  const { showSuccess, showError } = useSnackbar()

  /**
   * Copy text to clipboard
   * @param text Text to copy
   * @param message Success message to display (default: "Copied to clipboard")
   */
  const copy = async (text: string, message = 'Copied to clipboard'): Promise<void> => {
    try {
      await navigator.clipboard.writeText(text)
      showSuccess(message)
    } catch (error) {
      console.error('Failed to copy to clipboard:', error)
      showError('Failed to copy to clipboard')
    }
  }

  return { copy }
}
