pub mod token;

pub use token::TokenManager;

use crate::config::AppConfig;
use anyhow::{Context, Result, ensure};
use argon2::{Argon2, PasswordHash, PasswordVerifier};

/// Validate a password against the stored hash
pub fn validate_password(password: &str) -> Result<()> {
    ensure!(!password.is_empty(), "failed to validate password: empty");

    let password_hash = std::fs::read_to_string(&AppConfig::get().paths.password_file)
        .context("failed to read password file")?;

    ensure!(
        !password_hash.is_empty(),
        "failed to validate password: hash is empty"
    );

    let parsed_hash = PasswordHash::new(&password_hash)
        .map_err(|e| anyhow::anyhow!("failed to parse password hash: {e:#}"))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|e| anyhow::anyhow!("failed to verify password: {e:#}"))?;

    Ok(())
}
