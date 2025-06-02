export type FactoryReset = {
	keys: string[]
	result: FactoryResetResult
}

export type FactoryResetResult = {
	status: FactoryResetStatus
	context?: string | undefined
	error: string
	paths: string[]
}

export enum FactoryResetStatus {
	Unknown = -1,
	ModeSupported = 0,
	ModeUnsupported = 1,
	BackupRestoreError = 2,
	ConfigurationError = 3
}
