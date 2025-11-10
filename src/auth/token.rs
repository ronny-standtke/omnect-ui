use anyhow::Result;
use jwt_simple::prelude::*;
use std::sync::Arc;

const TOKEN_SUBJECT: &str = "omnect-ui";
const TOKEN_EXPIRE_HOURS: u64 = 2;
const TOKEN_TIME_TOLERANCE_MINS: u64 = 15;

/// Centralized token management for session tokens
///
/// Handles creation and verification of JWT tokens used for:
/// - Session authentication
/// - Centrifugo WebSocket authentication
///
/// This struct is cheap to clone (uses Arc internally) and can be safely
/// shared across threads and added to application data.
#[derive(Clone)]
pub struct TokenManager {
    inner: Arc<TokenManagerInner>,
}

struct TokenManagerInner {
    key: HS256Key,
}

impl TokenManager {
    /// Create a new TokenManager
    ///
    /// # Arguments
    /// * `secret` - Secret key for HMAC-SHA256 signing
    pub fn new(secret: &str) -> Self {
        Self {
            inner: Arc::new(TokenManagerInner {
                key: HS256Key::from_bytes(secret.as_bytes()),
            }),
        }
    }

    /// Create a new token with the configured expiration and subject
    ///
    /// Returns a signed JWT token string
    pub fn create_token(&self) -> Result<String> {
        let claims =
            Claims::create(Duration::from_hours(TOKEN_EXPIRE_HOURS)).with_subject(TOKEN_SUBJECT);

        self.inner
            .key
            .authenticate(claims)
            .map_err(|e| anyhow::anyhow!("failed to create token: {}", e))
    }

    /// Verify a token and check if it's valid
    ///
    /// Validates:
    /// - Signature
    /// - Expiration (with configurable time tolerance)
    /// - Max validity (token age)
    /// - Required subject claim
    ///
    /// Returns true if token is valid, false otherwise
    pub fn verify_token(&self, token: &str) -> bool {
        let options = VerificationOptions {
            accept_future: true,
            time_tolerance: Some(Duration::from_mins(TOKEN_TIME_TOLERANCE_MINS)),
            max_validity: Some(Duration::from_hours(TOKEN_EXPIRE_HOURS)),
            required_subject: Some(TOKEN_SUBJECT.to_string()),
            ..Default::default()
        };

        self.inner
            .key
            .verify_token::<NoCustomClaims>(token, Some(options))
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_verify_token() {
        let manager = TokenManager::new("test-secret");

        let token = manager.create_token().expect("should create token");
        assert!(!token.is_empty());

        assert!(manager.verify_token(&token));
    }

    #[test]
    fn test_verify_invalid_token() {
        let manager = TokenManager::new("test-secret");

        assert!(!manager.verify_token("invalid.token.here"));
        assert!(!manager.verify_token(""));
    }

    #[test]
    fn test_verify_token_wrong_secret() {
        let manager1 = TokenManager::new("secret1");
        let manager2 = TokenManager::new("secret2");

        let token = manager1.create_token().expect("should create token");

        // Token created with secret1 should not verify with secret2
        assert!(!manager2.verify_token(&token));
    }
}
