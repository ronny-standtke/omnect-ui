use serde::{Deserialize, Serialize};
use std::fmt;

use crate::types::*;

/// Authentication events
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum AuthEvent {
    Login {
        password: String,
    },
    Logout,
    SetPassword {
        password: String,
    },
    UpdatePassword {
        current_password: String,
        password: String,
    },
    CheckRequiresPasswordSet,
    #[serde(skip)]
    LoginResponse(Result<AuthToken, String>),
    #[serde(skip)]
    LogoutResponse(Result<(), String>),
    #[serde(skip)]
    SetPasswordResponse(Result<(), String>),
    #[serde(skip)]
    UpdatePasswordResponse(Result<(), String>),
    #[serde(skip)]
    CheckRequiresPasswordSetResponse(Result<bool, String>),
}

/// Device operation events
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum DeviceEvent {
    Reboot,
    FactoryResetRequest {
        mode: String,
        preserve: Vec<String>,
    },
    SetNetworkConfig {
        config: String,
    },
    NetworkFormStartEdit {
        adapter_name: String,
    },
    NetworkFormUpdate {
        form_data: String,
    },
    NetworkFormReset {
        adapter_name: String,
    },
    LoadUpdate {
        file_path: String,
    },
    UploadStarted,
    UploadProgress(u8),
    UploadCompleted(String),
    UploadFailed(String),
    RunUpdate {
        validate_iothub_connection: bool,
    },
    ReconnectionCheckTick,
    ReconnectionTimeout,
    NewIpCheckTick,
    NewIpCheckTimeout,
    AckRollback,
    #[serde(skip)]
    RebootResponse(Result<(), String>),
    #[serde(skip)]
    FactoryResetResponse(Result<(), String>),
    #[serde(skip)]
    SetNetworkConfigResponse(Result<crate::types::SetNetworkConfigResponse, String>),
    #[serde(skip)]
    LoadUpdateResponse(Result<UpdateManifest, String>),
    #[serde(skip)]
    RunUpdateResponse(Result<(), String>),
    #[serde(skip)]
    HealthcheckResponse(Result<HealthcheckInfo, String>),
    #[serde(skip)]
    AckRollbackResponse(Result<(), String>),
}

/// WebSocket/Centrifugo events
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum WebSocketEvent {
    SubscribeToChannels,
    UnsubscribeFromChannels,
    SystemInfoUpdated(SystemInfo),
    NetworkStatusUpdated(NetworkStatus),
    OnlineStatusUpdated(OnlineStatus),
    FactoryResetUpdated(FactoryReset),
    UpdateValidationStatusUpdated(UpdateValidationStatus),
    TimeoutsUpdated(Timeouts),
    Connected,
    Disconnected,
}

/// UI action events
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum UiEvent {
    ClearError,
    ClearSuccess,
    SetBrowserHostname(String),
}

/// Main event enum - wraps domain events
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Event {
    Initialize,
    Auth(AuthEvent),
    Device(DeviceEvent),
    WebSocket(WebSocketEvent),
    Ui(UiEvent),
}

/// Custom Debug implementation for AuthEvent to redact sensitive data
impl fmt::Debug for AuthEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthEvent::Login { .. } => f
                .debug_struct("Login")
                .field("password", &"<redacted>")
                .finish(),
            AuthEvent::SetPassword { .. } => f
                .debug_struct("SetPassword")
                .field("password", &"<redacted>")
                .finish(),
            AuthEvent::UpdatePassword { .. } => f
                .debug_struct("UpdatePassword")
                .field("current_password", &"<redacted>")
                .field("password", &"<redacted>")
                .finish(),
            AuthEvent::LoginResponse(result) => match result {
                Ok(_) => f
                    .debug_tuple("LoginResponse")
                    .field(&"Ok(<redacted token>)")
                    .finish(),
                Err(e) => f
                    .debug_tuple("LoginResponse")
                    .field(&format!("Err({e})"))
                    .finish(),
            },
            AuthEvent::Logout => write!(f, "Logout"),
            AuthEvent::CheckRequiresPasswordSet => write!(f, "CheckRequiresPasswordSet"),
            AuthEvent::LogoutResponse(r) => f.debug_tuple("LogoutResponse").field(r).finish(),
            AuthEvent::SetPasswordResponse(r) => {
                f.debug_tuple("SetPasswordResponse").field(r).finish()
            }
            AuthEvent::UpdatePasswordResponse(r) => {
                f.debug_tuple("UpdatePasswordResponse").field(r).finish()
            }
            AuthEvent::CheckRequiresPasswordSetResponse(r) => f
                .debug_tuple("CheckRequiresPasswordSetResponse")
                .field(r)
                .finish(),
        }
    }
}

/// Custom Debug implementation for Event
impl fmt::Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Event::Initialize => write!(f, "Initialize"),
            Event::Auth(e) => write!(f, "Auth({e:?})"),
            Event::Device(e) => write!(f, "Device({e:?})"),
            Event::WebSocket(e) => write!(f, "WebSocket({e:?})"),
            Event::Ui(e) => write!(f, "Ui({e:?})"),
        }
    }
}
