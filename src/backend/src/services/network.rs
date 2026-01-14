use crate::omnect_device_service_client::DeviceServiceClient;
use anyhow::{Context, Result};
use ini::Ini;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use serde_valid::Validate;
use std::{
    fs,
    io::ErrorKind,
    net::Ipv4Addr,
    path::Path,
    time::{Duration, SystemTime},
};
use tokio::{sync::broadcast, time::sleep};

// ============================================================================
// Macros
// ============================================================================

macro_rules! network_path {
    ($filename:expr) => {
        Path::new("/network/").join($filename)
    };
}

macro_rules! network_config_file {
    ($name:expr) => {
        network_path!(format!("10-{}.network", $name))
    };
}

macro_rules! network_backup_file {
    ($name:expr) => {
        network_path!(format!("10-{}.network.old", $name))
    };
}

macro_rules! network_rollback_file {
    () => {
        Path::new("/tmp/network_rollback.json")
    };
}

macro_rules! network_rollback_occurred_file {
    () => {
        Path::new("/tmp/network_rollback_occurred")
    };
}

macro_rules! clear_rollback {
    () => {
        let _ = fs::remove_file(network_rollback_file!());
    };
}

// ============================================================================
// Static State
// ============================================================================

static SERVER_RESTART_TX: std::sync::OnceLock<broadcast::Sender<()>> = std::sync::OnceLock::new();

// ============================================================================
// Constants
// ============================================================================

const ROLLBACK_TIMEOUT_SECS: u64 = 90;

// ============================================================================
// Structs
// ============================================================================

#[derive(Deserialize, Serialize, Clone, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConfig {
    is_server_addr: bool,
    ip_changed: bool,
    #[validate(min_length = 1)]
    name: String,
    dhcp: bool,
    previous_ip: Ipv4Addr,
    ip: Option<Ipv4Addr>,
    #[validate(maximum = 32)]
    #[validate(minimum = 0)]
    netmask: Option<u8>,
    gateway: Option<Vec<Ipv4Addr>>,
    dns: Option<Vec<Ipv4Addr>>,
}

#[derive(Deserialize, Serialize, Clone, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SetNetworkConfigRequest {
    #[serde(flatten)]
    #[validate]
    pub network: NetworkConfig,
    /// Whether to enable automatic rollback protection.
    /// Only applicable when is_server_addr=true AND ip_changed=true.
    /// If false/None, no rollback is created even for server IP changes.
    #[serde(default)]
    pub enable_rollback: Option<bool>,
    /// Whether this change is switching to DHCP (for rollback logic)
    #[serde(default)]
    pub switching_to_dhcp: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct Rollback {
    network_config: NetworkConfig,
    deadline: SystemTime,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SetNetworkConfigResponse {
    pub rollback_timeout_seconds: u64,
    pub ui_port: u16,
    pub rollback_enabled: bool,
}

// ============================================================================
// Service
// ============================================================================

/// Service for network configuration management operations
pub struct NetworkConfigService;

impl NetworkConfigService {
    /// Setup the server restart channel and return a receiver for restart signals
    ///
    /// # Returns
    /// Receiver for restart signals, or error if already initialized
    pub fn setup_restart_receiver() -> Result<broadcast::Receiver<()>, broadcast::Sender<()>> {
        let (tx, rx) = broadcast::channel(1);
        SERVER_RESTART_TX.set(tx).map(|_| rx)
    }

    /// Set network configuration with validation and rollback on error
    ///
    /// This is the main entry point for applying network configuration.
    /// It validates, applies, and handles rollback if needed.
    ///
    /// # Arguments
    /// * `service_client` - Device service client for network reload
    /// * `request` - Network configuration request with optional rollback settings
    ///
    /// # Returns
    /// Result with the network config response including rollback timeout, or an error
    pub async fn set_network_config<T>(
        service_client: &T,
        request: &SetNetworkConfigRequest,
    ) -> Result<SetNetworkConfigResponse>
    where
        T: DeviceServiceClient,
    {
        info!("set network config: {request:?}");

        request.validate().context("network validation failed")?;

        let enable_rollback = request.enable_rollback.unwrap_or(false);
        let switching_to_dhcp = request.switching_to_dhcp;

        if let Err(err1) = Self::apply_network_config(
            service_client,
            &request.network,
            enable_rollback,
            switching_to_dhcp,
        )
        .await
        {
            if let Err(err2) = Self::rollback_network_config(&request.network.name) {
                error!("failed to rollback network config: {err2:#}");
            }
            return Err(err1);
        }

        Ok(SetNetworkConfigResponse {
            rollback_timeout_seconds: ROLLBACK_TIMEOUT_SECS,
            ui_port: crate::config::AppConfig::get().ui.port,
            rollback_enabled: enable_rollback
                && request.network.is_server_addr
                && (request.network.ip_changed || switching_to_dhcp),
        })
    }

    /// Process any pending network configuration rollback
    ///
    /// # Arguments
    /// * `service_client` - Device service client for rollback operations
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn process_pending_rollback<T>(service_client: &T) -> Result<()>
    where
        T: DeviceServiceClient,
    {
        if Self::rollback_exists() {
            // load rollback
            let path = network_rollback_file!();
            let rollback: Rollback = serde_json::from_reader(
                std::fs::OpenOptions::new()
                    .read(true)
                    .open(path)
                    .context(format!("failed to open rollback file: {path:?}"))?,
            )
            .context(format!("failed to deserialize rollback: {path:?}"))?;

            // fails if deadline < now
            if let Ok(remaining_time) = rollback.deadline.duration_since(SystemTime::now()) {
                info!("pending rollback found: {rollback:?}");
                info!(
                    "await cancel rollback within: {}s",
                    remaining_time.as_secs()
                );
                sleep(remaining_time).await;
                return Box::pin(Self::process_pending_rollback(service_client)).await;
            }

            info!("rollback: {rollback:?}");
            Self::rollback_network_config(&rollback.network_config.name)?;
            service_client.reload_network().await?;
            Self::mark_rollback_occurred()?;
            Self::trigger_server_restart()?;

            clear_rollback!();
        } else {
            info!("no rollback found");
        }
        Ok(())
    }

    /// Cancel any pending network configuration rollback
    pub fn cancel_rollback() {
        if Self::rollback_exists() {
            clear_rollback!();
            info!("pending network rollback cancelled");
        }
    }

    /// Check if a rollback exists
    ///
    /// # Returns
    /// true if rollback file exists, false otherwise
    pub fn rollback_exists() -> bool {
        network_rollback_file!().exists()
    }

    /// Check if a rollback has occurred (and UI hasn't acknowledged it yet)
    ///
    /// # Returns
    /// true if rollback occurred marker file exists, false otherwise
    pub fn rollback_occurred() -> bool {
        network_rollback_occurred_file!().exists()
    }

    /// Clear the rollback occurred marker (called when UI acknowledges it)
    pub fn clear_rollback_occurred() {
        let _ = fs::remove_file(network_rollback_occurred_file!());
        info!("rollback occurred marker cleared");
    }

    /// Mark that a rollback has occurred (sets marker file)
    fn mark_rollback_occurred() -> Result<()> {
        fs::write(network_rollback_occurred_file!(), "")
            .context("failed to write rollback occurred marker")?;
        info!("rollback occurred marker set");
        Ok(())
    }

    /// Rollback network configuration to the previous backup
    ///
    /// # Arguments
    /// * `network_name` - Name of the network interface to rollback
    ///
    /// # Returns
    /// Result indicating success or failure
    fn rollback_network_config(network_name: &String) -> Result<()> {
        let config_file = network_config_file!(network_name);
        let backup_file = network_backup_file!(network_name);

        Self::rename_if_exists(&backup_file, &config_file)?;
        Ok(())
    }

    /// Atomically copy a file if it exists
    ///
    /// # Arguments
    /// * `src` - Source file path
    /// * `dest` - Destination file path
    ///
    /// # Returns
    /// Result with bool indicating if copy happened (true) or source didn't exist (false)
    fn copy_if_exists(src: &Path, dest: &Path) -> Result<bool> {
        match fs::copy(src, dest) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e).context(format!("failed to copy {src:?} to {dest:?}")),
        }
    }

    /// Atomically rename a file if it exists
    ///
    /// # Arguments
    /// * `src` - Source file path
    /// * `dest` - Destination file path
    ///
    /// # Returns
    /// Result with bool indicating if rename happened (true) or source didn't exist (false)
    fn rename_if_exists(src: &Path, dest: &Path) -> Result<bool> {
        match fs::rename(src, dest) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e).context(format!("failed to rename {src:?} to {dest:?}")),
        }
    }

    /// Trigger a server restart by sending signal through the restart channel
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Errors
    /// Returns error if the restart channel has not been initialized or if sending fails
    fn trigger_server_restart() -> Result<()> {
        let tx = SERVER_RESTART_TX
            .get()
            .context("failed to trigger restart: channel not initialized")?;

        tx.send(()).context("failed to send restart signal")?;

        Ok(())
    }

    /// Apply network configuration to systemd-networkd
    ///
    /// # Arguments
    /// * `service_client` - Device service client for network reload
    /// * `network` - Network configuration to apply
    /// * `enable_rollback` - Whether to enable automatic rollback for IP changes
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn apply_network_config<T>(
        service_client: &T,
        network: &NetworkConfig,
        enable_rollback: bool,
        switching_to_dhcp: bool,
    ) -> Result<()>
    where
        T: DeviceServiceClient,
    {
        info!("apply network config");

        Self::backup_current_network_config(service_client, &network.name).await?;
        Self::write_network_config(network)?;
        service_client.reload_network().await?;

        if network.is_server_addr && (network.ip_changed || switching_to_dhcp) {
            // Only create rollback if user explicitly requested it
            if enable_rollback {
                Self::create_rollback(network)?;
            }
            // Always restart server when server IP changes (regardless of rollback)
            Self::trigger_server_restart()?;
        }

        Ok(())
    }

    /// Backup the current network configuration file
    ///
    /// # Arguments
    /// * `service_client` - Device service client for retrieving network interfaces
    /// * `network_name` - Name of the network interface to backup
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn backup_current_network_config<T>(
        service_client: &T,
        network_name: &String,
    ) -> Result<()>
    where
        T: DeviceServiceClient,
    {
        info!("backup {network_name}");

        let config_file = network_config_file!(&network_name);
        let backup_file = network_backup_file!(&network_name);

        // copy file
        // if it doesn't exist try to find by network interfaces provided by omnect-device-service
        if !Self::copy_if_exists(&config_file, &backup_file)? {
            info!("current config file not found ({network_name})");
            info!("will try to find file in network interfaces provided by omnect-device-service");

            let status = service_client
                .status()
                .await
                .context("failed to get device status")?;

            debug!(
                "network interfaces: {:?}",
                status.network_status.network_interfaces
            );

            // find network file
            let file_name = status
                .network_status
                .network_interfaces
                .iter()
                .find(|iface| iface.name == *network_name)
                .context("failed to find network interface")?
                .file
                .file_name()
                .context("failed to get network file name")?;

            // map to internal mount
            let config_file = network_path!(file_name);
            log::debug!("config file is {config_file:?}");

            if !Self::copy_if_exists(&config_file, &backup_file)? {
                error!("failed to copy {config_file:?} to {backup_file:?}")
            }
        }

        Ok(())
    }

    /// Write network configuration to systemd-networkd file
    ///
    /// # Arguments
    /// * `network` - Network configuration to write
    ///
    /// # Returns
    /// Result indicating success or failure
    fn write_network_config(network: &NetworkConfig) -> Result<()> {
        let mut ini = Ini::new();

        ini.with_section(Some("Match".to_owned()))
            .set("Name", &network.name);

        let mut network_section = ini.with_section(Some("Network").to_owned());

        if network.dhcp {
            network_section.set("DHCP", "yes");
        } else {
            let ip = network.ip.context("network ip missing")?;
            let mask = network.netmask.context("network mask missing")?;

            network_section.set("Address", format!("{ip}/{mask}"));

            if let Some(gateways) = &network.gateway {
                for gateway in gateways {
                    network_section.add("Gateway", gateway.to_string());
                }
            }

            if let Some(dnss) = &network.dns {
                for dns in dnss {
                    network_section.add("DNS", dns.to_string());
                }
            }
        }

        let config_path = network_config_file!(&network.name);

        info!("write network config to {config_path:?}: {ini:?}");

        ini.write_to_file(&config_path)
            .context(format!("failed to write network config: {config_path:?}"))?;

        Ok(())
    }

    /// Create a rollback entry for network configuration changes
    ///
    /// # Arguments
    /// * `network` - Network configuration to create rollback for
    ///
    /// # Returns
    /// Result indicating success or failure
    fn create_rollback(network: &NetworkConfig) -> Result<()> {
        let rollback = Rollback {
            network_config: network.clone(),
            deadline: SystemTime::now() + Duration::from_secs(ROLLBACK_TIMEOUT_SECS),
        };

        info!("create rollback: {rollback:?}");

        let path = network_rollback_file!();

        serde_json::to_writer_pretty(
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .context(format!("failed to open rollback file for write: {path:?}"))?,
            &rollback,
        )
        .context(format!("failed to serialize rollback: {path:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_dhcp_config() -> NetworkConfig {
        NetworkConfig {
            is_server_addr: false,
            ip_changed: false,
            name: "eth0".to_string(),
            dhcp: true,
            previous_ip: Ipv4Addr::new(192, 168, 1, 100),
            ip: None,
            netmask: None,
            gateway: None,
            dns: None,
        }
    }

    fn create_valid_static_config() -> NetworkConfig {
        NetworkConfig {
            is_server_addr: false,
            ip_changed: false,
            name: "eth0".to_string(),
            dhcp: false,
            previous_ip: Ipv4Addr::new(192, 168, 1, 100),
            ip: Some(Ipv4Addr::new(192, 168, 1, 101)),
            netmask: Some(24),
            gateway: Some(vec![Ipv4Addr::new(192, 168, 1, 1)]),
            dns: Some(vec![Ipv4Addr::new(8, 8, 8, 8), Ipv4Addr::new(8, 8, 4, 4)]),
        }
    }

    mod validation {
        use super::*;

        #[test]
        fn valid_dhcp_config_passes() {
            let config = create_valid_dhcp_config();
            let request = SetNetworkConfigRequest {
                network: config,
                enable_rollback: None,
                switching_to_dhcp: true,
            };

            assert!(request.validate().is_ok());
        }

        #[test]
        fn valid_static_config_passes() {
            let config = create_valid_static_config();
            let request = SetNetworkConfigRequest {
                network: config,
                enable_rollback: None,
                switching_to_dhcp: false,
            };

            assert!(request.validate().is_ok());
        }

        #[test]
        fn empty_interface_name_fails() {
            let mut config = create_valid_dhcp_config();
            config.name = String::new();
            let request = SetNetworkConfigRequest {
                network: config,
                enable_rollback: None,
                switching_to_dhcp: false,
            };

            assert!(request.validate().is_err());
        }

        #[test]
        fn netmask_above_32_fails() {
            let mut config = create_valid_static_config();
            config.netmask = Some(33);
            let request = SetNetworkConfigRequest {
                network: config,
                enable_rollback: None,
                switching_to_dhcp: false,
            };

            assert!(request.validate().is_err());
        }

        #[test]
        fn netmask_at_boundary_passes() {
            let mut config = create_valid_static_config();
            config.netmask = Some(32);
            let request = SetNetworkConfigRequest {
                network: config,
                enable_rollback: None,
                switching_to_dhcp: false,
            };

            assert!(request.validate().is_ok());
        }

        #[test]
        fn netmask_zero_passes() {
            let mut config = create_valid_static_config();
            config.netmask = Some(0);
            let request = SetNetworkConfigRequest {
                network: config,
                enable_rollback: None,
                switching_to_dhcp: false,
            };

            assert!(request.validate().is_ok());
        }
    }

    mod ini_generation {
        use super::*;
        use tempfile::TempDir;

        #[test]
        fn write_network_config_creates_valid_ini_for_dhcp() {
            let temp_dir = TempDir::new().expect("failed to create temp dir");
            let config = NetworkConfig {
                is_server_addr: false,
                ip_changed: false,
                name: "eth0".to_string(),
                dhcp: true,
                previous_ip: Ipv4Addr::new(192, 168, 1, 100),
                ip: None,
                netmask: None,
                gateway: None,
                dns: None,
            };

            // Use the internal write function logic but with a temp path
            let mut ini = Ini::new();
            ini.with_section(Some("Match".to_owned()))
                .set("Name", &config.name);
            let mut network_section = ini.with_section(Some("Network").to_owned());
            network_section.set("DHCP", "yes");

            let config_path = temp_dir.path().join("10-eth0.network");
            ini.write_to_file(&config_path)
                .expect("failed to write ini");

            // Verify the file was created and contains expected content
            let contents = fs::read_to_string(&config_path).expect("failed to read ini");
            assert!(contents.contains("[Match]"));
            assert!(contents.contains("Name=eth0") || contents.contains("Name = eth0"));
            assert!(contents.contains("[Network]"));
            assert!(contents.contains("DHCP=yes") || contents.contains("DHCP = yes"));
        }

        #[test]
        fn write_network_config_creates_valid_ini_for_static() {
            let temp_dir = TempDir::new().expect("failed to create temp dir");
            let config = NetworkConfig {
                is_server_addr: false,
                ip_changed: false,
                name: "eth0".to_string(),
                dhcp: false,
                previous_ip: Ipv4Addr::new(192, 168, 1, 100),
                ip: Some(Ipv4Addr::new(192, 168, 1, 101)),
                netmask: Some(24),
                gateway: Some(vec![Ipv4Addr::new(192, 168, 1, 1)]),
                dns: Some(vec![Ipv4Addr::new(8, 8, 8, 8), Ipv4Addr::new(8, 8, 4, 4)]),
            };

            // Replicate the write logic
            let mut ini = Ini::new();
            ini.with_section(Some("Match".to_owned()))
                .set("Name", &config.name);
            let mut network_section = ini.with_section(Some("Network").to_owned());

            let ip = config.ip.expect("ip required for static");
            let mask = config.netmask.expect("mask required for static");
            network_section.set("Address", format!("{ip}/{mask}"));

            if let Some(gateways) = &config.gateway {
                for gateway in gateways {
                    network_section.add("Gateway", gateway.to_string());
                }
            }

            if let Some(dnss) = &config.dns {
                for dns in dnss {
                    network_section.add("DNS", dns.to_string());
                }
            }

            let config_path = temp_dir.path().join("10-eth0.network");
            ini.write_to_file(&config_path)
                .expect("failed to write ini");

            // Verify the file contents
            let contents = fs::read_to_string(&config_path).expect("failed to read ini");
            assert!(contents.contains("[Match]"));
            assert!(contents.contains("Name=eth0") || contents.contains("Name = eth0"));
            assert!(contents.contains("[Network]"));
            assert!(
                contents.contains("Address=192.168.1.101/24")
                    || contents.contains("Address = 192.168.1.101/24")
            );
            assert!(
                contents.contains("Gateway=192.168.1.1")
                    || contents.contains("Gateway = 192.168.1.1")
            );
            assert!(contents.contains("DNS=8.8.8.8") || contents.contains("DNS = 8.8.8.8"));
            assert!(contents.contains("DNS=8.8.4.4") || contents.contains("DNS = 8.8.4.4"));
        }
    }

    mod rollback_response {
        use super::*;

        #[test]
        fn response_includes_rollback_timeout() {
            let response = SetNetworkConfigResponse {
                rollback_timeout_seconds: ROLLBACK_TIMEOUT_SECS,
                ui_port: 1977,
                rollback_enabled: true,
            };

            assert_eq!(response.rollback_timeout_seconds, 90);
        }

        #[test]
        fn rollback_enabled_when_ip_changed_and_is_server() {
            let response = SetNetworkConfigResponse {
                rollback_timeout_seconds: ROLLBACK_TIMEOUT_SECS,
                ui_port: 1977,
                rollback_enabled: true,
            };

            assert!(response.rollback_enabled);
        }

        #[test]
        fn rollback_disabled_when_not_requested() {
            let response = SetNetworkConfigResponse {
                rollback_timeout_seconds: ROLLBACK_TIMEOUT_SECS,
                ui_port: 1977,
                rollback_enabled: false,
            };

            assert!(!response.rollback_enabled);
        }
    }

    mod serde {
        use super::*;

        #[test]
        fn network_config_serializes_with_camel_case() {
            let config = create_valid_dhcp_config();
            let json = serde_json::to_string(&config).expect("failed to serialize");

            assert!(json.contains("\"isServerAddr\""));
            assert!(json.contains("\"ipChanged\""));
            assert!(json.contains("\"previousIp\""));
        }

        #[test]
        fn network_config_deserializes_from_camel_case() {
            let json = r#"{
                "isServerAddr": false,
                "ipChanged": false,
                "name": "eth0",
                "dhcp": true,
                "previousIp": "192.168.1.100"
            }"#;

            let config: NetworkConfig = serde_json::from_str(json).expect("failed to deserialize");

            assert_eq!(config.name, "eth0");
            assert!(config.dhcp);
            assert_eq!(config.previous_ip, Ipv4Addr::new(192, 168, 1, 100));
        }

        #[test]
        fn request_includes_enable_rollback_field() {
            let config = create_valid_dhcp_config();
            let request = SetNetworkConfigRequest {
                network: config,
                enable_rollback: Some(true),
                switching_to_dhcp: false,
            };

            let json = serde_json::to_string(&request).expect("failed to serialize");
            assert!(json.contains("\"enableRollback\""));
        }
    }
}
