use serde::{Deserialize, Serialize};

use crate::capabilities::centrifugo::CentrifugoOutput;
use crate::types::*;

/// Events that can happen in the app
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Event {
    // Initialization
    Initialize,

    // Authentication
    Login {
        password: String,
    },
    Logout,
    SetPassword {
        password: String,
    },
    UpdatePassword {
        current: String,
        new_password: String,
    },
    CheckRequiresPasswordSet,

    // Device actions
    Reboot,
    FactoryResetRequest {
        mode: String,
        preserve: Vec<String>,
    },
    ReloadNetwork,

    // Network configuration
    SetNetworkConfig {
        config: String,
    },

    // Update actions
    LoadUpdate {
        file_path: String,
    },
    RunUpdate {
        validate_iothub: bool,
    },

    // WebSocket subscriptions
    SubscribeToChannels,
    UnsubscribeFromChannels,

    // WebSocket updates (from Centrifugo)
    SystemInfoUpdated(SystemInfo),
    NetworkStatusUpdated(NetworkStatus),
    OnlineStatusUpdated(OnlineStatus),
    FactoryResetUpdated(FactoryReset),
    UpdateValidationStatusUpdated(UpdateValidationStatus),
    TimeoutsUpdated(Timeouts),

    // HTTP responses (internal events, skipped from serialization)
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
    #[serde(skip)]
    RebootResponse(Result<(), String>),
    #[serde(skip)]
    FactoryResetResponse(Result<(), String>),
    #[serde(skip)]
    ReloadNetworkResponse(Result<(), String>),
    #[serde(skip)]
    SetNetworkConfigResponse(Result<(), String>),
    #[serde(skip)]
    LoadUpdateResponse(Result<(), String>),
    #[serde(skip)]
    RunUpdateResponse(Result<(), String>),
    #[serde(skip)]
    HealthcheckResponse(Result<HealthcheckInfo, String>),

    // Connection state
    Connected,
    Disconnected,

    // Centrifugo responses (internal events)
    #[serde(skip)]
    CentrifugoResponse(CentrifugoOutput),

    // UI actions
    ClearError,
    ClearSuccess,
}
