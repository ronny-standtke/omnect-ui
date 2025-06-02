use crate::{omnect_device_service_client::OmnectDeviceServiceClient, socket_client::SocketClient};
use anyhow::{Context, Result};
use log::info;
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
    let uri = std::env::var("IOTEDGE_WORKLOADURI").context("IOTEDGE_WORKLOADURI missing")?;
    let payload = CreateCertPayload {
        common_name: ods_client.ip_address().await?,
    };
    let path = format!("/modules/{id}/genid/{gen_id}/certificate/server?api-version={api_version}");
    let uri = hyperlocal::Uri::new(
        uri.strip_prefix("unix://")
            .context("unexpected workload uri prefix")?,
        &path,
    )
    .into();
    let socket_client = SocketClient::new();
    let response: CreateCertResponse = serde_json::from_str(
        &socket_client
            .post_with_json_body(&uri, payload)
            .await
            .context("create_module_certificate request failed")?,
    )
    .context("CreateCertResponse not possible")?;
    let mut file = File::create(cert_path()).context("could not be create cert_path")?;
    file.write_all(response.certificate.as_bytes())
        .context("could not write to cert_path")?;

    let mut file = File::create(key_path()).context("could not be create key_path")?;
    file.write_all(response.private_key.bytes.as_bytes())
        .context("could not write to key_path")
}
