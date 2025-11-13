#![cfg_attr(feature = "mock", allow(dead_code, unused_imports))]

use crate::{
    certificate::CreateCertPayload,
    config::AppConfig,
    http_client::{handle_http_response, unix_socket_client},
};
use anyhow::{Context, Result, anyhow, bail};
use log::info;
#[cfg(feature = "mock")]
use mockall::automock;
use reqwest::Client;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::{env, fmt::Debug, path::PathBuf, sync::OnceLock};
use trait_variant::make;

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
    pub update_file_path: PathBuf,
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

#[derive(Deserialize, Debug)]
pub struct NetworkInterface {
    pub online: bool,
    pub ipv4: Ipv4Info,
    pub file: String,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct Ipv4Info {
    pub addrs: Vec<Ipv4AddrInfo>,
}

#[derive(Deserialize, Debug)]
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

#[derive(Clone, Debug, Serialize)]
pub struct HeaderKeyValue {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublishEndpoint {
    pub url: String,
    pub headers: Vec<HeaderKeyValue>,
}

#[derive(Debug, Serialize)]
struct PublishIdEndpoint {
    id: &'static str,
    endpoint: PublishEndpoint,
}

#[derive(Clone)]
pub struct OmnectDeviceServiceClient {
    client: Client,
    has_publish_endpoint: bool,
}

type CertSetupFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>>>>;
type CertSetupFn = Box<dyn FnOnce(CreateCertPayload) -> CertSetupFuture>;

#[derive(Default)]
pub struct OmnectDeviceServiceClientBuilder {
    publish_endpoint: Option<PublishEndpoint>,
    certificate_setup: Option<CertSetupFn>,
}

impl OmnectDeviceServiceClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_publish_endpoint(mut self, endpoint: PublishEndpoint) -> Self {
        self.publish_endpoint = Some(endpoint);
        self
    }

    pub fn with_certificate_setup<F, Fut>(mut self, setup_fn: F) -> Self
    where
        F: FnOnce(CreateCertPayload) -> Fut + 'static,
        Fut: std::future::Future<Output = Result<()>> + 'static,
    {
        self.certificate_setup = Some(Box::new(move |payload| Box::pin(setup_fn(payload))));
        self
    }

    pub async fn build(self) -> Result<OmnectDeviceServiceClient> {
        let client = unix_socket_client(
            &AppConfig::get()
                .device_service
                .socket_path
                .to_string_lossy(),
        )?;

        let mut omnect_client = OmnectDeviceServiceClient {
            client,
            has_publish_endpoint: false,
        };

        // Setup certificate if provided
        if let Some(setup_fn) = self.certificate_setup {
            let common_name = omnect_client.ip_address().await?;
            let payload = CreateCertPayload { common_name };
            setup_fn(payload).await?;
        }

        // Register publish endpoint if provided
        if let Some(endpoint) = self.publish_endpoint {
            omnect_client.register_publish_endpoint(endpoint).await?;
            omnect_client.has_publish_endpoint = true;
        }

        Ok(omnect_client)
    }
}

#[make(Send)]
#[cfg_attr(feature = "mock", automock)]
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
    async fn shutdown(&self) -> Result<()>;
}

impl OmnectDeviceServiceClient {
    const REQUIRED_CLIENT_VERSION: &str = ">=0.39.0";

    // API endpoint constants
    const STATUS_ENDPOINT: &str = "/status/v1";
    const REPUBLISH_ENDPOINT: &str = "/republish/v1/";
    const FACTORY_RESET_ENDPOINT: &str = "/factory-reset/v1";
    const REBOOT_ENDPOINT: &str = "/reboot/v1";
    const RELOAD_NETWORK_ENDPOINT: &str = "/reload-network/v1";
    const LOAD_UPDATE_ENDPOINT: &str = "/fwupdate/load/v1";
    const RUN_UPDATE_ENDPOINT: &str = "/fwupdate/run/v1";
    const PUBLISH_ENDPOINT: &str = "/publish-endpoint/v1";

    fn required_version() -> &'static VersionReq {
        static REQUIRED_VERSION: OnceLock<VersionReq> = OnceLock::new();
        REQUIRED_VERSION.get_or_init(|| {
            VersionReq::parse(Self::REQUIRED_CLIENT_VERSION)
                .expect("invalid REQUIRED_CLIENT_VERSION constant")
        })
    }

    async fn register_publish_endpoint(&self, endpoint: PublishEndpoint) -> Result<()> {
        let publish_id_endpoint = PublishIdEndpoint {
            id: env!("CARGO_PKG_NAME"),
            endpoint,
        };
        self.post_json(Self::PUBLISH_ENDPOINT, publish_id_endpoint)
            .await?;
        Ok(())
    }

    fn build_url(&self, path: &str) -> String {
        // Normalize path to always start with a single "/"
        let normalized_path = path.trim_start_matches('/');
        format!("http://localhost/{normalized_path}")
    }

    /// GET request to the device service API
    async fn get(&self, path: &str) -> Result<String> {
        let url = self.build_url(path);
        info!("GET {url}");

        let res = self
            .client
            .get(&url)
            .send()
            .await
            .context(format!("failed to send GET request to {url}"))?;

        handle_http_response(res, &format!("GET {url}")).await
    }

    /// POST request to the device service API (empty body)
    async fn post(&self, path: &str) -> Result<String> {
        let url = self.build_url(path);
        info!("POST {url}");

        let res = self
            .client
            .post(&url)
            .send()
            .await
            .context(format!("failed to send POST request to {url}"))?;

        handle_http_response(res, &format!("POST {url}")).await
    }

    /// POST request to the device service API with JSON body
    async fn post_json(&self, path: &str, body: impl Debug + Serialize) -> Result<String> {
        let url = self.build_url(path);
        info!("POST {url} with body: {body:?}");

        let res = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context(format!("failed to send POST request to {url}"))?;

        handle_http_response(res, &format!("POST {url}")).await
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
            .find_map(|iface| {
                iface
                    .online
                    .then(|| iface.ipv4.addrs.first().map(|addr| addr.addr.clone()))
                    .flatten()
            })
            .context("failed to get ip address from status")
    }

    async fn status(&self) -> Result<Status> {
        let body = self.get(Self::STATUS_ENDPOINT).await?;
        serde_json::from_str(&body).context("failed to parse status")
    }

    async fn republish(&self) -> Result<()> {
        self.post(&format!(
            "{}{}",
            Self::REPUBLISH_ENDPOINT,
            env!("CARGO_PKG_NAME")
        ))
        .await?;
        Ok(())
    }

    async fn factory_reset(&self, factory_reset: FactoryReset) -> Result<()> {
        self.post_json(Self::FACTORY_RESET_ENDPOINT, factory_reset)
            .await?;
        Ok(())
    }

    async fn reboot(&self) -> Result<()> {
        self.post(Self::REBOOT_ENDPOINT).await?;
        Ok(())
    }

    async fn reload_network(&self) -> Result<()> {
        self.post(Self::RELOAD_NETWORK_ENDPOINT).await?;
        Ok(())
    }

    async fn load_update(&self, load_update: LoadUpdate) -> Result<String> {
        self.post_json(Self::LOAD_UPDATE_ENDPOINT, load_update)
            .await
    }

    async fn run_update(&self, run_update: RunUpdate) -> Result<()> {
        self.post_json(Self::RUN_UPDATE_ENDPOINT, run_update)
            .await?;
        Ok(())
    }

    async fn healthcheck_info(&self) -> Result<HealthcheckInfo> {
        let status = self.status().await?;
        let current_version = status.system_info.omnect_device_service_version;

        let required_version = Self::required_version();
        let parsed_current = Version::parse(&current_version)
            .map_err(|e| anyhow!("failed to parse current version: {e}"))?;

        Ok(HealthcheckInfo {
            version_info: VersionInfo {
                required: Self::REQUIRED_CLIENT_VERSION.to_string(),
                current: current_version,
                mismatch: !required_version.matches(&parsed_current),
            },
            update_validation_status: status.update_validation_status,
        })
    }

    async fn shutdown(&self) -> Result<()> {
        if self.has_publish_endpoint {
            let path = format!("{}/{}", Self::PUBLISH_ENDPOINT, env!("CARGO_PKG_NAME"));
            let url = self.build_url(&path);
            info!("DELETE {url}");

            self.client
                .delete(&url)
                .send()
                .await
                .context("failed to send DELETE request to unregister endpoint")?
                .error_for_status()
                .context("failed to unregister endpoint: server returned error status")?;
        }
        Ok(())
    }
}
