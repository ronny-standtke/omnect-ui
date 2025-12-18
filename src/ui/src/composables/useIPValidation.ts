/**
 * Composable for IP address and network validation
 *
 * @returns Object with validation functions
 *
 * @example
 * ```ts
 * const { isValidIp, isValidNetmask } = useIPValidation()
 *
 * <v-text-field :rules="[isValidIp]" />
 * ```
 */
export function useIPValidation() {
  const IPV4_REGEX = /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}$/

  /**
   * Validate IPv4 address format
   * @param value IP address to validate
   * @returns true if valid, error message if invalid
   */
  const isValidIp = (value: string): boolean | string => {
    if (!value) return true
    return IPV4_REGEX.test(value) || "Invalid IPv4-Address"
  }

  /**
   * Validate and parse netmask value
   * @param mask Netmask string (e.g., "/24" or "24")
   * @returns Prefix length if valid, null if invalid
   */
  const parseNetmask = (mask: string): number | null => {
    const prefixLen = Number.parseInt(mask.replace("/", ""), 10)
    if (isNaN(prefixLen) || prefixLen < 0 || prefixLen > 32) {
      return null
    }
    return prefixLen
  }

  /**
   * Validate netmask format
   * @param mask Netmask string
   * @returns true if valid, error message if invalid
   */
  const isValidNetmask = (mask: string): boolean | string => {
    const prefixLen = parseNetmask(mask)
    return prefixLen !== null || "Invalid netmask"
  }

  return {
    isValidIp,
    parseNetmask,
    isValidNetmask
  }
}
