use crate::socket_client;
use actix_web::body::MessageBody;
use anyhow::{anyhow, Context, Result};
use log::{debug, info};
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

#[derive(Deserialize)]
struct StatusResponse {
    #[serde(rename = "NetworkStatus")]
    network_status: NetworkStatus,
}

#[derive(Deserialize)]
struct NetworkStatus {
    #[serde(rename = "network_status")]
    network_interfaces: Vec<NetworkInterface>,
}

#[derive(Deserialize)]
struct NetworkInterface {
    online: bool,
    ipv4: Ipv4Info,
}

#[derive(Deserialize)]
struct Ipv4Info {
    addrs: Vec<Ipv4AddrInfo>,
}

#[derive(Deserialize)]
struct Ipv4AddrInfo {
    addr: String,
}

#[cfg(feature = "mock")]
pub async fn create_module_certificate(_cert_path: &str, _key_path: &str) -> Result<()> {
    Ok(())
}

#[cfg(not(feature = "mock"))]
pub async fn create_module_certificate(cert_path: &str, key_path: &str) -> Result<()> {
    info!("create module certificate");

    let iotedge_moduleid = std::env::var("IOTEDGE_MODULEID").context("IOTEDGE_MODULEID missing")?;
    let iotedge_modulegenerationid = std::env::var("IOTEDGE_MODULEGENERATIONID")
        .context("IOTEDGE_MODULEGENERATIONID missing")?;
    let iotedge_apiversion =
        std::env::var("IOTEDGE_APIVERSION").context("IOTEDGE_APIVERSION missing")?;
    let iotedge_workloaduri =
        std::env::var("IOTEDGE_WORKLOADURI").context("IOTEDGE_WORKLOADURI missing")?;

    let ods_socket_path = std::env::var("SOCKET_PATH").context("env SOCKET_PATH is missing")?;
    let ip = get_ip_address(&ods_socket_path).await?;
    debug!("IP address: {}", ip);

    let payload = CreateCertPayload { common_name: ip };
    let path = format!("/modules/{iotedge_moduleid}/genid/{iotedge_modulegenerationid}/certificate/server?api-version={iotedge_apiversion}");
    let socket_path = iotedge_workloaduri
        .strip_prefix("unix://")
        .context("failed to strip socket path")?;

    let response = socket_client::post_with_json_body(&path, payload, socket_path)
        .await
        .context("create_module_certificate request failed")?;

    let cert_response: CreateCertResponse = serde_json::from_slice(
        &response
            .into_body()
            .try_into_bytes()
            .map_err(|e| anyhow!("Failed to convert response body into bytes: {e:?}"))?,
    )
    .context("CreateCertResponse not possible")?;

    let mut file = File::create(cert_path).context("could not be create cert_path")?;
    file.write_all(cert_response.certificate.as_bytes())
        .context("could not write to cert_path")?;

    let mut file = File::create(key_path).context("could not be create key_path")?;
    file.write_all(cert_response.private_key.bytes.as_bytes())
        .context("could not write to key_path")
}

async fn get_ip_address(ods_socket_path: &str) -> Result<String> {
    let response = socket_client::get_with_empty_body("/status/v1", ods_socket_path)
        .await
        .context("Failed to get status from socket client")?;
    let body_bytes = response
        .into_body()
        .try_into_bytes()
        .map_err(|e| anyhow!("Failed to convert response body into bytes: {e:?}"))?;

    let status_response: StatusResponse =
        serde_json::from_slice(&body_bytes).context("Failed to parse StatusResponse from JSON")?;

    status_response
        .network_status
        .network_interfaces
        .into_iter()
        .find(|iface| iface.online)
        .and_then(|iface| iface.ipv4.addrs.into_iter().next())
        .map(|addr_info| addr_info.addr)
        .ok_or_else(|| anyhow!("No online network interface with IPv4 address found"))
}
