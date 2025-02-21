export type IpAddress = {
	addr: string
	dhcp: boolean
	prefix_len: number
}

export type InternetProtocol = {
	addrs: IpAddress[]
	dns: string[]
	gateways: string[]
}

export type DeviceNetwork = {
	ipv4: InternetProtocol
	mac: string
	name: string
	online: boolean
}

export type NetworkStatus = {
	network_status: DeviceNetwork[]
}
