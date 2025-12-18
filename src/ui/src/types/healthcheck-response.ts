export type HealthcheckResponse = {
	version_info: VersionInfo
	update_validation_status: UpdateValidationStatus
	network_rollback_occurred: boolean
}

export type VersionInfo = {
	required: string
	current: string
	mismatch: boolean
}

export type UpdateValidationStatus = {
	status: string
}
