use crate::keycloak_client;
use anyhow::{Context, Result, bail, ensure};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use reqwest::Response;
use std::{
    env::var,
    io::Write,
    sync::{Arc, OnceLock},
};
use uuid::Uuid;

#[derive(Clone)]
pub struct CentrifugoConfig {
    pub port: String,
    pub client_token: String,
    pub api_key: String,
}

pub fn centrifugo_config() -> Arc<CentrifugoConfig> {
    static CENTRIFUGO_CONFIG: OnceLock<Arc<CentrifugoConfig>> = OnceLock::new();
    CENTRIFUGO_CONFIG
        .get_or_init(|| {
            let port = var("CENTRIFUGO_HTTP_SERVER_PORT").unwrap_or_else(|_| "8000".to_string());
            let client_token = Uuid::new_v4().to_string();
            let api_key = Uuid::new_v4().to_string();

            Arc::new(CentrifugoConfig {
                port,
                client_token,
                api_key,
            })
        })
        .clone()
}

pub fn centrifugo_publish_endpoint() -> crate::omnect_device_service_client::PublishEndpoint {
    let cfg = centrifugo_config();
    crate::omnect_device_service_client::PublishEndpoint {
        url: format!("https://localhost:{}/api/publish", cfg.port),
        headers: vec![
            crate::omnect_device_service_client::HeaderKeyValue {
                name: String::from("Content-Type"),
                value: String::from("application/json"),
            },
            crate::omnect_device_service_client::HeaderKeyValue {
                name: String::from("X-API-Key"),
                value: cfg.api_key.clone(),
            },
        ],
    }
}

macro_rules! config_path {
    () => {
        std::path::Path::new(
            &std::env::var("CONFIG_PATH").unwrap_or_else(|_| "/data/config".to_string()),
        )
    };
    ($filename:expr) => {
        std::path::Path::new(
            &std::env::var("CONFIG_PATH").unwrap_or_else(|_| "/data/config".to_string()),
        )
        .join($filename)
    };
}
pub(crate) use config_path;

macro_rules! data_path {
    ($filename:expr) => {
        std::path::Path::new("/data/").join($filename)
    };
}
pub(crate) use data_path;

macro_rules! host_data_path {
    ($filename:expr) => {
        std::path::Path::new(&format!("/var/lib/{}/", env!("CARGO_PKG_NAME"))).join($filename)
    };
}
pub(crate) use host_data_path;

macro_rules! tmp_path {
    ($filename:expr) => {
        std::path::Path::new("/tmp/").join($filename)
    };
}
pub(crate) use tmp_path;

pub fn validate_password(password: &str) -> Result<()> {
    if password.is_empty() {
        bail!("failed to validate password: empty");
    }

    let Ok(password_hash) = std::fs::read_to_string(config_path!("password")) else {
        bail!("failed to read password file");
    };

    if password_hash.is_empty() {
        bail!("failed to validate password: hash is empty");
    }

    let Ok(parsed_hash) = PasswordHash::new(&password_hash) else {
        bail!("failed to parse password hash");
    };

    if let Err(e) = Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
        bail!("failed to verify password: {e}");
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

/// Handle HTTP response by checking status and extracting body
pub async fn handle_http_response(res: Response, context_msg: &str) -> Result<String> {
    let status = res.status();
    let body = res.text().await.context("failed to read response body")?;

    ensure!(
        status.is_success(),
        "{context_msg} failed with status {status} and body: {body}"
    );

    Ok(body)
}
