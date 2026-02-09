use serde::{Deserialize, Serialize};
use serde_valid::Validate;

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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate)]
pub struct SetPasswordRequest {
    #[validate(min_length = 1)]
    pub password: String,
}

/// Request to update existing password
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePasswordRequest {
    #[validate(min_length = 1)]
    pub current_password: String,
    #[validate(min_length = 1)]
    pub password: String,
}
