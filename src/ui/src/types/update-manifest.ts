export type UpdateManifest = {
	readonly updateId: UpdateId
	readonly isDeployable: boolean
	readonly compatibility: readonly Compatibility[]
	readonly createdDateTime: string
	readonly manifestVersion: string
}

export type UpdateId = {
	readonly provider: string
	readonly name: string
	readonly version: string
}

export type Compatibility = {
	readonly manufacturer: string
	readonly model: string
	readonly compatibilityid: string
}
