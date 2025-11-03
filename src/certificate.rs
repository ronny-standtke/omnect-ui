#![cfg_attr(feature = "mock", allow(dead_code, unused_imports))]

use crate::omnect_device_service_client::{DeviceServiceClient, OmnectDeviceServiceClient};
use anyhow::{Context, Result, ensure};
use log::info;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Write};

#[derive(Serialize)]
struct CreateCertPayload {
    #[serde(rename = "commonName")]
    common_name: String,
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
    std::env::var("CERT_PATH").unwrap_or("/cert/cert.pem".to_string())
}

pub fn key_path() -> String {
    std::env::var("KEY_PATH").unwrap_or("/cert/key.pem".to_string())
}

#[cfg(feature = "mock")]
pub async fn create_module_certificate() -> Result<()> {
    Ok(())
}

#[cfg(not(feature = "mock"))]
pub async fn create_module_certificate() -> Result<()> {
    info!("create module certificate");
    let ods_client = OmnectDeviceServiceClient::new(false).await?;
    let id = std::env::var("IOTEDGE_MODULEID").context("IOTEDGE_MODULEID missing")?;
    let gen_id = std::env::var("IOTEDGE_MODULEGENERATIONID")
        .context("IOTEDGE_MODULEGENERATIONID missing")?;
    let api_version = std::env::var("IOTEDGE_APIVERSION").context("IOTEDGE_APIVERSION missing")?;
    let workload_uri =
        std::env::var("IOTEDGE_WORKLOADURI").context("IOTEDGE_WORKLOADURI missing")?;

    let payload = CreateCertPayload {
        common_name: ods_client.ip_address().await?,
    };

    let path = format!("/modules/{id}/genid/{gen_id}/certificate/server?api-version={api_version}");

    // Extract the Unix socket path from the workload URI
    // IoT Edge provides URIs like "unix:///var/run/iotedge/workload.sock"
    let socket_path = workload_uri
        .strip_prefix("unix://")
        .context("IOTEDGE_WORKLOADURI must use unix:// scheme")?;

    // Create a client for the IoT Edge workload socket
    let client = Client::builder()
        .unix_socket(socket_path)
        .build()
        .context("failed to create HTTP client for workload socket")?;

    let url = format!("http://localhost{}", path);
    info!("POST {} (IoT Edge workload API)", url);

    let res = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .context("failed to send certificate request to IoT Edge workload API")?;

    let status = res.status();
    let body = res.text().await.context("failed to read response body")?;

    ensure!(
        status.is_success(),
        "certificate request failed with status {} and body: {}",
        status,
        body
    );

    let response: CreateCertResponse =
        serde_json::from_str(&body).context("failed to parse CreateCertResponse")?;

    let mut file = File::create(cert_path()).context("failed to create cert file")?;
    file.write_all(response.certificate.as_bytes())
        .context("failed to write certificate to file")?;

    let mut file = File::create(key_path()).context("failed to create key file")?;
    file.write_all(response.private_key.bytes.as_bytes())
        .context("failed to write private key to file")
}
