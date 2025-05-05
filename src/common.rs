use std::path::Path;

use anyhow::{bail, Result};
use argon2::{Argon2, PasswordHash, PasswordVerifier};

macro_rules! config_path {
    ($filename:expr) => {{
        static CONFIG_PATH_DEFAULT: &'static str = "/data/config";
        Path::new(&std::env::var("CONFIG_PATH").unwrap_or(CONFIG_PATH_DEFAULT.to_string()))
            .join($filename)
    }};
}
pub(crate) use config_path;

pub fn validate_password(password: &str) -> Result<()> {
    if password.is_empty() {
        bail!("password is empty");
    }

    let password_file = config_path!("password");

    let Ok(password_hash) = std::fs::read_to_string(password_file) else {
        bail!("failed to read password file");
    };

    if password_hash.is_empty() {
        bail!("password hash is empty");
    }

    let Ok(parsed_hash) = PasswordHash::new(&password_hash) else {
        bail!("failed to parse password hash");
    };

    if let Err(e) = Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
        bail!("password verification failed: {e}");
    }

    Ok(())
}
