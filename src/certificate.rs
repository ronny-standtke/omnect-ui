#![cfg_attr(feature = "mock", allow(dead_code, unused_imports))]

use crate::{common::handle_http_response, http_client};
use anyhow::{Context, Result};
use log::info;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Write};

// Public payload for passing to certificate creation
#[derive(Debug, Serialize)]
pub struct CreateCertPayload {
    #[serde(rename = "commonName")]
    pub common_name: String,
}

#[derive(Debug, Deserialize)]
struct PrivateKey {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    type_name: String,
    bytes: String,
}

#[derive(Debug, Deserialize)]
struct CreateCertResponse {
    #[serde(rename = "privateKey")]
    private_key: PrivateKey,
    certificate: String,
    #[allow(dead_code)]
    expiration: String,
}

pub fn cert_path() -> String {
    std::env::var("CERT_PATH").unwrap_or_else(|_| "/cert/cert.pem".to_string())
}

pub fn key_path() -> String {
    std::env::var("KEY_PATH").unwrap_or_else(|_| "/cert/key.pem".to_string())
}

#[cfg(feature = "mock")]
pub async fn create_module_certificate(_payload: CreateCertPayload) -> Result<()> {
    Ok(())
}

#[cfg(not(feature = "mock"))]
pub async fn create_module_certificate(payload: CreateCertPayload) -> Result<()> {
    info!("create module certificate");
    let id = std::env::var("IOTEDGE_MODULEID")
        .context("failed to read IOTEDGE_MODULEID environment variable")?;
    let gen_id = std::env::var("IOTEDGE_MODULEGENERATIONID")
        .context("failed to read IOTEDGE_MODULEGENERATIONID environment variable")?;
    let api_version = std::env::var("IOTEDGE_APIVERSION")
        .context("failed to read IOTEDGE_APIVERSION environment variable")?;
    let workload_uri = std::env::var("IOTEDGE_WORKLOADURI")
        .context("failed to read IOTEDGE_WORKLOADURI environment variable")?;

    let path = format!("modules/{id}/genid/{gen_id}/certificate/server?api-version={api_version}");

    // Create a client for the IoT Edge workload socket
    let client = http_client::unix_socket_client(&workload_uri)?;

    let url = format!("http://localhost/{path}");
    info!("POST {url} (IoT Edge workload API) with payload: {payload:?}");

    let res = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .context("failed to send certificate request to IoT Edge workload API")?;

    let body = handle_http_response(res, "certificate request").await?;

    let response: CreateCertResponse =
        serde_json::from_str(&body).context("failed to parse CreateCertResponse")?;

    let mut file = File::create(cert_path()).context("failed to create cert file")?;
    file.write_all(response.certificate.as_bytes())
        .context("failed to write certificate to file")?;

    let mut file = File::create(key_path()).context("failed to create key file")?;
    file.write_all(response.private_key.bytes.as_bytes())
        .context("failed to write private key to file")
}
