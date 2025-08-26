#![cfg_attr(feature = "mock", allow(dead_code, unused_imports))]

use crate::{common::centrifugo_config, socket_client::SocketClient};
use anyhow::{Context, Result, anyhow, bail};
use hyperlocal::Uri;
#[cfg(feature = "mock")]
use mockall::automock;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::env;

#[derive(Clone, Debug, Default, Deserialize_repr, PartialEq, Serialize_repr)]
#[repr(u8)]
pub enum FactoryResetMode {
    #[default]
    Mode1 = 1,
    Mode2 = 2,
    Mode3 = 3,
    Mode4 = 4,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct FactoryReset {
    mode: FactoryResetMode,
    preserve: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadUpdate {
    pub update_file_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunUpdate {
    validate_iothub_connection: bool,
}

#[derive(Deserialize)]
pub struct Status {
    #[serde(rename = "NetworkStatus")]
    pub network_status: NetworkStatus,
    #[serde(rename = "SystemInfo")]
    pub system_info: SystemInfo,
    #[serde(rename = "UpdateValidationStatus")]
    pub update_validation_status: UpdateValidationStatus,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UpdateValidationStatus {
    pub status: String,
}

#[derive(Deserialize)]
pub struct SystemInfo {
    pub fleet_id: Option<String>,
    pub omnect_device_service_version: String,
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

#[derive(Clone, Debug, Serialize)]
pub struct VersionInfo {
    pub required: String,
    pub current: String,
    pub mismatch: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct HealthcheckInfo {
    pub version_info: VersionInfo,
    pub update_validation_status: UpdateValidationStatus,
}

#[derive(Serialize)]
struct HeaderKeyValue {
    name: String,
    value: String,
}

#[derive(Serialize)]
struct PublishEndpoint {
    url: String,
    headers: Vec<HeaderKeyValue>,
}

#[derive(Serialize)]
struct PublishIdEndpoint {
    id: &'static str,
    endpoint: PublishEndpoint,
}

#[derive(Clone)]
pub struct OmnectDeviceServiceClient {
    socket_client: SocketClient,
    socket_path: String,
    register_publish_endpoint: bool,
}

#[cfg_attr(feature = "mock", automock)]
#[allow(async_fn_in_trait)]
pub trait DeviceServiceClient {
    async fn fleet_id(&self) -> Result<String>;

    async fn ip_address(&self) -> Result<String>;

    async fn status(&self) -> Result<Status>;

    async fn republish(&self) -> Result<()>;

    async fn factory_reset(&self, factory_reset: FactoryReset) -> Result<()>;

    async fn reboot(&self) -> Result<()>;

    async fn reload_network(&self) -> Result<()>;

    async fn load_update(&self, load_update: LoadUpdate) -> Result<String>;

    async fn run_update(&self, run_update: RunUpdate) -> Result<()>;

    async fn healthcheck_info(&self) -> Result<HealthcheckInfo>;
}

impl OmnectDeviceServiceClient {
    const REQUIRED_CLIENT_VERSION: &str = ">=0.39.0";

    pub async fn new(register_publish_endpoint: bool) -> Result<Self> {
        let socket_client = SocketClient::new();
        let socket_path = env::var("SOCKET_PATH").unwrap_or("/socket/api.sock".to_string());

        let client = OmnectDeviceServiceClient {
            socket_client,
            socket_path,
            register_publish_endpoint,
        };

        if register_publish_endpoint {
            client.register_publish_endpoint().await?;
        }
        Ok(client)
    }

    async fn register_publish_endpoint(&self) -> Result<()> {
        let centrifugo_config = centrifugo_config();

        let headers = vec![
            HeaderKeyValue {
                name: String::from("Content-Type"),
                value: String::from("application/json"),
            },
            HeaderKeyValue {
                name: String::from("X-API-Key"),
                value: centrifugo_config.api_key,
            },
        ];

        let body = PublishIdEndpoint {
            id: env!("CARGO_PKG_NAME"),
            endpoint: PublishEndpoint {
                url: format!("https://localhost:{}/api/publish", centrifugo_config.port),
                headers,
            },
        };

        self.post_with_json_body("/publish-endpoint/v1", body)
            .await
            .map(|_| ())
    }

    async fn post_with_empty_body(&self, path: &str) -> Result<String> {
        self.socket_client
            .post_with_empty_body(&Uri::new(&self.socket_path, path).into())
            .await
    }

    async fn post_with_json_body(&self, path: &str, body: impl Serialize) -> Result<String> {
        self.socket_client
            .post_with_json_body(&Uri::new(&self.socket_path, path).into(), body)
            .await
    }
}

impl DeviceServiceClient for OmnectDeviceServiceClient {
    async fn fleet_id(&self) -> Result<String> {
        let status = self.status().await?;

        let Some(fleet_id) = status.system_info.fleet_id else {
            bail!("failed to get fleet id from status")
        };

        Ok(fleet_id)
    }

    async fn ip_address(&self) -> Result<String> {
        // we return the first online ipv4 address that was found
        self.status()
            .await?
            .network_status
            .network_interfaces
            .iter()
            .filter_map(|iface| {
                if iface.online {
                    if let Some(addr_info) = iface.ipv4.addrs.first() {
                        return Some(addr_info.addr.clone());
                    }
                }
                None
            })
            .next()
            .context("failed to get ip address from status")
    }

    async fn status(&self) -> Result<Status> {
        let body = self
            .socket_client
            .get_with_empty_body(&Uri::new(&self.socket_path, "/status/v1").into())
            .await?;
        serde_json::from_str(&body).context("failed to parse status")
    }

    async fn republish(&self) -> Result<()> {
        self.post_with_empty_body(concat!("/republish/v1/", env!("CARGO_PKG_NAME")))
            .await
            .map(|_| ())
    }

    async fn factory_reset(&self, factory_reset: FactoryReset) -> Result<()> {
        self.post_with_json_body("/factory-reset/v1", factory_reset)
            .await
            .map(|_| ())
    }

    async fn reboot(&self) -> Result<()> {
        self.post_with_empty_body("/reboot/v1").await.map(|_| ())
    }

    async fn reload_network(&self) -> Result<()> {
        self.post_with_empty_body("/reload-network/v1")
            .await
            .map(|_| ())
    }

    async fn load_update(&self, load_update: LoadUpdate) -> Result<String> {
        self.post_with_json_body("/fwupdate/load/v1", load_update)
            .await
    }

    async fn run_update(&self, run_update: RunUpdate) -> Result<()> {
        self.post_with_json_body("/fwupdate/run/v1", run_update)
            .await
            .map(|_| ())
    }

    async fn healthcheck_info(&self) -> Result<HealthcheckInfo> {
        let status = self.status().await?;
        let current_version = status.system_info.omnect_device_service_version;

        let required_version = VersionReq::parse(Self::REQUIRED_CLIENT_VERSION)
            .map_err(|e| anyhow!("failed to parse required version: {e}"))?;
        let current_version = Version::parse(&current_version)
            .map_err(|e| anyhow!("failed to parse current version: {e}"))?;

        Ok(HealthcheckInfo {
            version_info: VersionInfo {
                required: Self::REQUIRED_CLIENT_VERSION.to_string(),
                current: current_version.to_string(),
                mismatch: !required_version.matches(&current_version),
            },
            update_validation_status: status.update_validation_status,
        })
    }
}

impl Drop for OmnectDeviceServiceClient {
    fn drop(&mut self) {
        if self.register_publish_endpoint {
            let socket_client = self.socket_client.clone();
            let socket_path = self.socket_path.clone();

            tokio::spawn(async move {
                socket_client
                    .delete_with_empty_body(
                        &Uri::new(
                            &socket_path,
                            concat!("/publish-endpoint/v1/", env!("CARGO_PKG_NAME")),
                        )
                        .into(),
                    )
                    .await
            });
        }
    }
}
