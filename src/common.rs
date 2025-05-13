use actix_web::body::MessageBody;
use anyhow::{anyhow, bail, Context, Result};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::{prelude::BASE64_STANDARD, Engine};
use jwt_simple::prelude::{RS256PublicKey, RSAPublicKeyLike};
use reqwest::blocking::get;
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};

#[derive(Deserialize)]
pub struct RealmInfo {
    public_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct TokenClaims {
    roles: Option<Vec<String>>,
    tenant_list: Option<Vec<String>>,
    fleet_list: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct StatusResponse {
    #[serde(rename = "NetworkStatus")]
    pub network_status: NetworkStatus,
    #[serde(rename = "SystemInfo")]
    pub system_info: SystemInfo,
}

#[derive(Deserialize)]
pub struct SystemInfo {
    pub fleet_id: Option<String>,
}

#[derive(Deserialize)]
pub struct NetworkStatus {
    #[serde(rename = "network_status")]
    pub network_interfaces: Vec<NetworkInterface>,
}

#[derive(Deserialize)]
pub struct NetworkInterface {
    pub online: bool,
    pub ipv4: Ipv4Info,
}

#[derive(Deserialize)]
pub struct Ipv4Info {
    pub addrs: Vec<Ipv4AddrInfo>,
}

#[derive(Deserialize)]
pub struct Ipv4AddrInfo {
    pub addr: String,
}

#[derive(Serialize, Deserialize)]
struct FrontEndConfig {
    #[serde(rename = "KEYCLOAK_URL")]
    keycloak_url: String,
}

macro_rules! config_path {
    ($filename:expr) => {{
        static CONFIG_PATH_DEFAULT: &'static str = "/data/config";
        Path::new(&std::env::var("CONFIG_PATH").unwrap_or(CONFIG_PATH_DEFAULT.to_string()))
            .join($filename)
    }};
}
pub(crate) use config_path;

use crate::socket_client;

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

pub async fn validate_token_and_claims(
    token: &str,
    keycloak_public_key_url: &str,
    tenant: &String,
    ods_socket_path: &str,
) -> Result<()> {
    let pub_key = get_keycloak_realm_public_key(keycloak_public_key_url)
        .await
        .context("failed to get public key")?;

    let claims = pub_key
        .verify_token::<TokenClaims>(token, None)
        .context("failed to verify token")?;

    let Some(tenant_list) = &claims.custom.tenant_list else {
        bail!("user has no tenant list");
    };

    if !tenant_list.contains(tenant) {
        bail!("user has no permission to set password");
    }

    let Some(roles) = &claims.custom.roles else {
        bail!("user has no roles");
    };

    if roles.contains(&String::from("FleetAdministrator")) {
        return Ok(());
    }

    if roles.contains(&String::from("FleetOperator")) {
        let Some(fleet_list) = &claims.custom.fleet_list else {
            bail!("user has no permission on this fleet");
        };

        let fleet_id = get_fleet_id(ods_socket_path)
            .await
            .context("failed to get fleet id")?;

        if !fleet_list.contains(&fleet_id) {
            bail!("user has no permission on this fleet");
        } else {
            return Ok(());
        }
    }

    bail!("user has no permission to set password")
}

async fn get_keycloak_realm_public_key(keycloak_public_key_url: &str) -> Result<RS256PublicKey> {
    let resp = get(keycloak_public_key_url)
        .context("failed to fetch from url")?
        .json::<RealmInfo>()
        .context("failed to parse realm info")?;

    RS256PublicKey::from_der(&BASE64_STANDARD.decode(resp.public_key.as_bytes()).unwrap())
        .context("failed to decode public key")
}

async fn get_fleet_id(ods_socket_path: &str) -> Result<String> {
    let status_response: StatusResponse = get_status(ods_socket_path)
        .await
        .context("failed to parse StatusResponse from JSON")?;

    let Some(fleet_id) = &status_response.system_info.fleet_id else {
        bail!("failed to get fleet id from status response")
    };

    Ok(fleet_id.clone())
}

pub async fn get_status(ods_socket_path: &str) -> Result<StatusResponse> {
    let response = socket_client::get_with_empty_body("/status/v1", ods_socket_path)
        .await
        .context("failed to get status from socket client")?;
    let body_bytes = response
        .into_body()
        .try_into_bytes()
        .map_err(|e| anyhow!("failed to convert response body into bytes: {e:?}"))?;

    serde_json::from_slice(&body_bytes).context("failed to parse StatusResponse from JSON")
}

pub fn create_frontend_config_file(keycloak_url: &str) -> Result<()> {
    let config_path = config_path!("app_config.js");
    let Some(parent) = config_path.parent() else {
        bail!("failed to get parent directory for frontend config file")
    };

    fs::create_dir_all(parent).context("failed to create frontend config directory")?;

    let mut config_file =
        std::fs::File::create(config_path).context("failed to create frontend config file")?;

    config_file
        .write_all(
            format!("window.__APP_CONFIG__ = {{KEYCLOAK_URL:\"{keycloak_url}\"}};").as_bytes(),
        )
        .unwrap();

    Ok(())
}
