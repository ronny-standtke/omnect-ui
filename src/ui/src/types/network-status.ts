export type IpAddress = {
	readonly addr: string
	readonly dhcp: boolean
	readonly prefix_len: number
}

export type InternetProtocol = {
	readonly addrs: readonly IpAddress[]
	readonly dns: readonly string[]
	readonly gateways: readonly string[]
}

export type DeviceNetwork = {
	readonly ipv4: InternetProtocol
	readonly mac: string
	readonly name: string
	readonly online: boolean
}

export type NetworkStatus = {
	readonly network_status: readonly DeviceNetwork[]
}
