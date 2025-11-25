//! Password management service
//!
//! Handles password hashing, storage, and validation independent of HTTP concerns.

use crate::config::AppConfig;
use anyhow::{Context, Result, anyhow, ensure};
use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use log::debug;
use std::{fs::File, io::Write};

/// Service for password management operations
pub struct PasswordService;

impl PasswordService {
    /// Validate a password against the stored hash
    ///
    /// # Arguments
    /// * `password` - The plaintext password to validate
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn validate_password(password: &str) -> Result<()> {
        ensure!(!password.is_empty(), "failed to validate password: empty");

        let password_file = &AppConfig::get().paths.password_file;
        let password_hash =
            std::fs::read_to_string(password_file).context("failed to read password file")?;

        ensure!(
            !password_hash.is_empty(),
            "failed to validate password: hash is empty"
        );

        let parsed_hash = PasswordHash::new(&password_hash)
            .map_err(|e| anyhow!(e))
            .context("failed to parse password hash")?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|e| anyhow!(e))
            .context("failed to verify password")
    }

    /// Hash a password using Argon2
    fn hash_password(password: &str) -> Result<String> {
        debug!("hash_password() called");

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| anyhow!(e))
            .context("failed to hash password")
    }

    /// Store or update a password
    ///
    /// # Arguments
    /// * `password` - The plaintext password to store
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn store_or_update_password(password: &str) -> Result<()> {
        debug!("store_or_update_password() called");

        let password_file = &AppConfig::get().paths.password_file;
        let hash = Self::hash_password(password)?;
        let mut file = File::create(password_file).context("failed to create password file")?;

        file.write_all(hash.as_bytes())
            .context("failed to write password file")
    }

    /// Check if a password has been set
    ///
    /// # Returns
    /// true if password file exists, false otherwise
    pub fn password_exists() -> bool {
        AppConfig::get()
            .paths
            .password_file
            .try_exists()
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password() {
        let hash = PasswordService::hash_password("testpassword").expect("should hash");
        assert!(!hash.is_empty());
        assert!(hash.starts_with("$argon2"));
    }

    #[test]
    fn test_store_and_check_password() {
        // This test relies on AppConfig which is initialized in test mode with temp directories
        // Clean up any existing password file first
        let password_file = &AppConfig::get().paths.password_file;
        let _ = std::fs::remove_file(password_file);

        assert!(!PasswordService::password_exists());

        PasswordService::store_or_update_password("testpass").expect("should store password");

        assert!(PasswordService::password_exists());

        // Cleanup
        let _ = std::fs::remove_file(password_file);
    }
}
