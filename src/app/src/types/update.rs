use serde::{Deserialize, Serialize};

/// Update validation status from WebSocket
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateValidationStatus {
    pub status: String,
}

/// Version information for healthcheck
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionInfo {
    pub required: String,
    pub current: String,
    pub mismatch: bool,
}

/// Healthcheck response
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthcheckInfo {
    pub version_info: VersionInfo,
    pub update_validation_status: UpdateValidationStatus,
    #[serde(default)]
    pub network_rollback_occurred: bool,
}

/// Request to load update manifest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoadUpdateRequest {
    pub file_path: String,
}

/// Request to run update
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunUpdateRequest {
    pub validate_iothub_connection: bool,
}

/// Update identifier
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateId {
    pub provider: String,
    pub name: String,
    pub version: String,
}

/// Compatibility information
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Compatibility {
    pub manufacturer: String,
    pub model: String,
    pub compatibilityid: String,
}

/// Update manifest loaded from file
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    pub update_id: UpdateId,
    pub is_deployable: bool,
    pub compatibility: Vec<Compatibility>,
    pub created_date_time: String,
    pub manifest_version: String,
}

/// State of the firmware upload
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum UploadState {
    #[default]
    Idle,
    Uploading,
    Completed,
    Failed(String),
}
