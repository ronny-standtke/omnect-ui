use serde::{Deserialize, Serialize};

/// Authentication token returned from login
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthToken {
    pub token: String,
}

/// Login credentials
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginCredentials {
    pub password: String,
}

/// Request to set initial password
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetPasswordRequest {
    pub password: String,
}

/// Request to update existing password
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePasswordRequest {
    pub current_password: String,
    pub password: String,
}
