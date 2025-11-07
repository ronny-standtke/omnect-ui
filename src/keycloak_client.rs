use anyhow::{Context, Result};
use base64::{Engine, prelude::BASE64_STANDARD};
use jwt_simple::prelude::{RS256PublicKey, RSAPublicKeyLike};
#[cfg(feature = "mock")]
use mockall::automock;
use serde::{Deserialize, Serialize};

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

#[derive(Clone)]
pub struct KeycloakProvider {
    client: reqwest::Client,
}

impl Default for KeycloakProvider {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl KeycloakProvider {
    async fn realm_public_key(&self) -> Result<RS256PublicKey> {
        let resp = self
            .client
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
}

impl SingleSignOnProvider for KeycloakProvider {
    async fn verify_token(&self, token: &str) -> anyhow::Result<TokenClaims> {
        let pub_key = self.realm_public_key().await?;
        let claims = pub_key.verify_token::<TokenClaims>(token, None)?;
        Ok(claims.custom)
    }
}

pub fn config() -> String {
    let keycloak_url = &keycloak_url!();
    format!("window.__APP_CONFIG__ = {{KEYCLOAK_URL:\"{keycloak_url}\"}};")
}
