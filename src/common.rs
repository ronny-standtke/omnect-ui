use crate::keycloak_client;
use anyhow::{Context, Result, bail};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use serde::{Deserialize, Serialize};
use std::{env::var, io::Write, sync::OnceLock};
use uuid::Uuid;

static CENTRIFUGO_CONFIG: OnceLock<CentrifugoConfig> = OnceLock::new();

#[derive(Clone)]
pub struct CentrifugoConfig {
    pub port: String,
    pub client_token: String,
    pub api_key: String,
}

pub fn centrifugo_config() -> CentrifugoConfig {
    CENTRIFUGO_CONFIG
        .get_or_init(|| {
            let port = var("CENTRIFUGO_HTTP_SERVER_PORT").unwrap_or("8000".to_string());
            let client_token = Uuid::new_v4().to_string();
            let api_key = Uuid::new_v4().to_string();

            CentrifugoConfig {
                port,
                client_token,
                api_key,
            }
        })
        .clone()
}

#[derive(Serialize, Deserialize)]
struct FrontEndConfig {
    #[serde(rename = "KEYCLOAK_URL")]
    keycloak_url: String,
}

macro_rules! config_path {
    () => {
        std::path::Path::new(&std::env::var("CONFIG_PATH").unwrap_or("/data/config".to_string()))
    };
    ($filename:expr) => {
        std::path::Path::new(&std::env::var("CONFIG_PATH").unwrap_or("/data/config".to_string()))
            .join($filename)
    };
}
pub(crate) use config_path;

pub fn validate_password(password: &str) -> Result<()> {
    if password.is_empty() {
        bail!("password is empty");
    }

    let Ok(password_hash) = std::fs::read_to_string(config_path!("password")) else {
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

pub fn create_frontend_config_file() -> Result<()> {
    let mut config_file = std::fs::File::create(config_path!("app_config.js"))
        .context("failed to create frontend config file")?;

    config_file
        .write_all(keycloak_client::config().as_bytes())
        .context("failed to write frontend config file")
}
