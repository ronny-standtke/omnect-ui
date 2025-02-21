export type SystemInfo = {
	os: {
		name: string
		version: string
	}
	azure_sdk_version: string
	omnect_device_service_version: string
	boot_time?: string
}
