use serde::{Deserialize, Serialize};
use std::fmt;

/// Factory reset operation status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum FactoryResetStatus {
    #[default]
    Unknown,
    ModeSupported,
    ModeUnsupported,
    BackupRestoreError,
    ConfigurationError,
}

impl fmt::Display for FactoryResetStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => write!(f, "unknown"),
            Self::ModeSupported => write!(f, "modeSupported"),
            Self::ModeUnsupported => write!(f, "modeUnsupported"),
            Self::BackupRestoreError => write!(f, "backupRestoreError"),
            Self::ConfigurationError => write!(f, "configurationError"),
        }
    }
}

/// Result of factory reset operation
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FactoryResetResult {
    pub status: FactoryResetStatus,
    pub context: Option<String>,
    pub error: String,
    pub paths: Vec<String>,
}

/// Factory reset state from WebSocket
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FactoryReset {
    pub keys: Vec<String>,
    #[serde(default)]
    pub result: Option<FactoryResetResult>,
}

/// Request to initiate factory reset
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FactoryResetRequest {
    pub mode: u8,
    pub preserve: Vec<String>,
}
