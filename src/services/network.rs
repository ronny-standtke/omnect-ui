use crate::omnect_device_service_client::DeviceServiceClient;
use anyhow::{Context, Result};
use ini::Ini;
use log::{error, info};
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

macro_rules! save_rollback {
    ($rollback:expr) => {
        (|| -> Result<()> {
            let rollback_json =
                serde_json::to_string_pretty($rollback).context("failed to serialize rollback")?;
            fs::write(network_rollback_file!(), rollback_json)
                .context("failed to write rollback file")
        })()
    };
}

macro_rules! load_rollback {
    () => {
        fs::read_to_string(network_rollback_file!())
            .ok()
            .and_then(|contents| serde_json::from_str::<PendingRollback>(&contents).ok())
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

#[derive(Deserialize, Serialize, Clone, Validate)]
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

#[derive(Deserialize, Serialize, Clone)]
struct PendingRollback {
    network_config: NetworkConfig,
    rollback_time: SystemTime,
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
    /// * `network` - Network configuration to apply
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn set_network_config<T>(service_client: &T, network: &NetworkConfig) -> Result<()>
    where
        T: DeviceServiceClient,
    {
        network.validate().context("validation failed")?;

        if let Err(e) = Self::apply_network_config(service_client, network).await {
            if let Err(err) = Self::rollback_network_config(network) {
                error!("failed to restore network config: {err:#}");
            }
            return Err(e);
        }

        Ok(())
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
        if let Some(pending) = load_rollback!() {
            if let Ok(remaining_time) = pending.rollback_time.duration_since(SystemTime::now()) {
                sleep(remaining_time).await;
            }

            if load_rollback!().is_some() {
                Self::execute_rollback(service_client, &pending.network_config, "scheduled").await;
                clear_rollback!();
            }
        }
        Ok(())
    }

    /// Cancel any pending network configuration rollback
    pub fn cancel_rollback() {
        if load_rollback!().is_some() {
            clear_rollback!();
            info!("pending network rollback cancelled");
        }
    }

    /// Rollback network configuration to the previous backup
    ///
    /// # Arguments
    /// * `network` - Network configuration to rollback
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn rollback_network_config(network: &NetworkConfig) -> Result<()> {
        let config_file = network_config_file!(network.name);
        let backup_file = network_backup_file!(network.name);

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
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn apply_network_config<T>(service_client: &T, network: &NetworkConfig) -> Result<()>
    where
        T: DeviceServiceClient,
    {
        Self::backup_current_network_config(service_client, network).await?;
        Self::write_network_config(network)?;
        service_client.reload_network().await?;

        if network.is_server_addr && network.ip_changed {
            Self::schedule_server_restart(network).await?;
        }

        Ok(())
    }

    async fn backup_current_network_config<T>(
        service_client: &T,
        network: &NetworkConfig,
    ) -> Result<()>
    where
        T: DeviceServiceClient,
    {
        let config_file = network_config_file!(&network.name);
        let backup_file = network_backup_file!(&network.name);

        if !Self::copy_if_exists(&config_file, &backup_file)? {
            let status = service_client
                .status()
                .await
                .context("failed to get status")?;

            let current_network = status
                .network_status
                .network_interfaces
                .iter()
                .find(|iface| iface.name == network.name)
                .context("failed to find current network interface")?;

            log::debug!("current network is {current_network:?}");

            let file_name = Path::new(&current_network.file)
                .file_name()
                .context("context")?;

            let config_file = network_path!(file_name);
            log::debug!("config file is {config_file:?}");

            Self::copy_if_exists(&config_file, &backup_file)?;
        }

        Ok(())
    }

    async fn rollback_and_restart<T>(service_client: &T, network: &NetworkConfig) -> Result<()>
    where
        T: DeviceServiceClient,
    {
        Self::rollback_network_config(network)?;
        service_client.reload_network().await?;
        Self::trigger_server_restart()?;

        Ok(())
    }

    async fn execute_rollback<T>(service_client: &T, network: &NetworkConfig, label: &str)
    where
        T: DeviceServiceClient,
    {
        info!("executing {} network rollback", label);

        if let Err(e) = Self::rollback_and_restart(service_client, network).await {
            error!("failed to execute {label} rollback: {e:#}");
        } else {
            info!("{} network rollback executed successfully", label);
        }
    }

    fn write_network_config(network: &NetworkConfig) -> Result<()> {
        let mut ini = Ini::new();

        ini.with_section(Some("Match".to_owned()))
            .set("Name", &network.name);

        let mut network_section = ini.with_section(Some("Network").to_owned());

        if network.dhcp {
            network_section.set("DHCP", "yes");
        } else {
            network_section.set(
                "Address",
                format!("{}/{}", network.ip.unwrap(), network.netmask.unwrap()),
            );

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
        ini.write_to_file(&config_path).context(format!(
            "failed to write network config file {config_path:?}"
        ))?;

        Ok(())
    }

    async fn schedule_server_restart(network: &NetworkConfig) -> Result<()> {
        let rollback_time = SystemTime::now() + Duration::from_secs(ROLLBACK_TIMEOUT_SECS);

        let pending_rollback = PendingRollback {
            network_config: network.clone(),
            rollback_time,
        };

        if let Err(e) = save_rollback!(&pending_rollback) {
            error!("failed to save pending rollback: {e:#}");
        }

        Self::trigger_server_restart()?;

        Ok(())
    }
}
