/**
 * TypeScript type definitions for ODS (omnect-device-service) JSON payloads
 * These types represent the JSON format sent over Centrifugo WebSocket
 */

export interface OdsOnlineStatus {
  iothub: boolean
}

export interface OdsOsInfo {
  name: string
  version: string
}

export interface OdsSystemInfo {
  os: OdsOsInfo
  azure_sdk_version: string
  omnect_device_service_version: string
  boot_time: number | null
}

export interface OdsDuration {
  nanos: number
  secs: number
}

export interface OdsTimeouts {
  wait_online_timeout: OdsDuration
}

export interface OdsIpAddress {
  addr: string
  dhcp: boolean
  prefix_len: number
}

export interface OdsInternetProtocol {
  addrs: OdsIpAddress[]
  dns: string[]
  gateways: string[]
}

export interface OdsDeviceNetwork {
  ipv4: OdsInternetProtocol
  mac: string
  name: string
  online: boolean
  file: string | null
}

export interface OdsNetworkStatus {
  network_status: OdsDeviceNetwork[]
}

export interface OdsFactoryResetResult {
  status: 'unknown' | 'mode_supported' | 'mode_unsupported' | 'backup_restore_error' | 'configuration_error'
  context: string | null
  error: string
  paths: string[]
}

export interface OdsFactoryReset {
  keys: string[]
  result: OdsFactoryResetResult | null
}

export interface OdsUpdateValidationStatus {
  status: string
}
