use anyhow::{Context, Result};
use base64::{prelude::BASE64_STANDARD, Engine};
use jwt_simple::prelude::RS256PublicKey;
use reqwest::blocking::get;
use serde::Deserialize;

#[derive(Deserialize)]
struct RealmInfo {
    public_key: String,
}

macro_rules! keycloak_url {
    () => {{
        std::env::var("KEYCLOAK_URL")
            .unwrap_or("https://keycloak.omnect.conplement.cloud/realms/cp-prod".to_string())
    }};
}

pub fn config() -> String {
    let keycloak_url = &keycloak_url!();
    format!("window.__APP_CONFIG__ = {{KEYCLOAK_URL:\"{keycloak_url}\"}};")
}

pub async fn realm_public_key() -> Result<RS256PublicKey> {
    let resp = get(keycloak_url!())
        .context("failed to fetch from url")?
        .json::<RealmInfo>()
        .context("failed to parse realm info")?;

    RS256PublicKey::from_der(&BASE64_STANDARD.decode(resp.public_key.as_bytes()).unwrap())
        .context("failed to decode public key")
}
