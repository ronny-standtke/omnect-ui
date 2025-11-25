export type UpdateManifest = {
	updateId: UpdateId
	isDeployable: boolean
	compatibility: Compatibility[]
	createdDateTime: string
	manifestVersion: string
}

export type UpdateId = {
	provider: string
	name: string
	version: string
}

export type Compatibility = {
	manufacturer: string
	model: string
	compatibilityid: string
}
