use anyhow::{Context, Result};
use base64::{Engine, prelude::BASE64_STANDARD};
use jwt_simple::prelude::{RS256PublicKey, RSAPublicKeyLike};
#[cfg(feature = "mock")]
use mockall::automock;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenClaims {
    pub roles: Option<Vec<String>>,
    pub tenant_list: Option<Vec<String>>,
    pub fleet_list: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct RealmInfo {
    public_key: String,
}

macro_rules! keycloak_url {
    () => {{
        std::env::var("KEYCLOAK_URL").unwrap_or_else(|_| {
            "https://keycloak.omnect.conplement.cloud/realms/cp-prod".to_string()
        })
    }};
}

#[cfg_attr(feature = "mock", automock)]
#[allow(async_fn_in_trait)]
pub trait SingleSignOnProvider: Send + Sync {
    async fn verify_token(&self, token: &str) -> anyhow::Result<TokenClaims>;
}

#[derive(Clone, Default)]
pub struct KeycloakProvider;

impl SingleSignOnProvider for KeycloakProvider {
    async fn verify_token(&self, token: &str) -> anyhow::Result<TokenClaims> {
        let pub_key = crate::keycloak_client::realm_public_key().await?;
        let claims = pub_key.verify_token::<TokenClaims>(token, None)?;
        Ok(claims.custom)
    }
}

pub fn config() -> String {
    let keycloak_url = &keycloak_url!();
    format!("window.__APP_CONFIG__ = {{KEYCLOAK_URL:\"{keycloak_url}\"}};")
}

fn http_client() -> &'static Client {
    static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
    HTTP_CLIENT.get_or_init(Client::new)
}

pub async fn realm_public_key() -> Result<RS256PublicKey> {
    let client = http_client();
    let resp = client
        .get(keycloak_url!())
        .send()
        .await
        .context("failed to fetch from url")?
        .json::<RealmInfo>()
        .await
        .context("failed to parse realm info")?;

    let decoded = BASE64_STANDARD
        .decode(resp.public_key.as_bytes())
        .context("failed to decode public key from base64")?;

    RS256PublicKey::from_der(&decoded).context("failed to parse public key from DER format")
}
