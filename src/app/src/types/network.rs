use serde::{Deserialize, Serialize};

/// Validate IPv4 address format
pub fn is_valid_ipv4(ip: &str) -> bool {
    if ip.is_empty() {
        return true; // Empty is considered valid (for optional fields)
    }

    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        return false;
    }

    parts.iter().all(|part| {
        if let Ok(num) = part.parse::<u32>() {
            num <= 255
        } else {
            false
        }
    })
}

/// Validate and parse netmask value
/// Accepts "/24" or "24" format, returns prefix length if valid
pub fn parse_netmask(mask: &str) -> Option<u32> {
    let cleaned = mask.trim_start_matches('/');
    if let Ok(prefix_len) = cleaned.parse::<u32>() {
        if prefix_len <= 32 {
            return Some(prefix_len);
        }
    }
    None
}

/// IP address configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct IpAddress {
    pub addr: String,
    pub dhcp: bool,
    pub prefix_len: u32,
}

/// Internet protocol configuration (IPv4/IPv6)
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct InternetProtocol {
    pub addrs: Vec<IpAddress>,
    pub dns: Vec<String>,
    pub gateways: Vec<String>,
}

/// Network adapter information
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceNetwork {
    pub ipv4: InternetProtocol,
    pub mac: String,
    pub name: String,
    pub online: bool,
    #[serde(default)]
    pub file: Option<String>,
}

/// Network status from WebSocket
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkStatus {
    pub network_status: Vec<DeviceNetwork>,
}

impl NetworkStatus {
    /// Determine which adapter is the current connection based on browser hostname
    pub fn current_connection_adapter(
        &self,
        browser_hostname: Option<&str>,
    ) -> Option<&DeviceNetwork> {
        let hostname = browser_hostname?;

        // First, try to find a direct IP match
        for adapter in &self.network_status {
            for ip in &adapter.ipv4.addrs {
                if ip.addr == hostname {
                    return Some(adapter);
                }
            }
        }

        // Special case: if we are on localhost, and an adapter has "localhost" IP, match it
        if hostname == "localhost" {
            for adapter in &self.network_status {
                if adapter.ipv4.addrs.iter().any(|ip| ip.addr == "localhost") {
                    return Some(adapter);
                }
            }
        }

        // If hostname is a domain name (not an IP), we can't determine which adapter
        // is the current connection without DNS resolution, so return None
        None
    }
}

/// Network configuration request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConfigRequest {
    pub is_server_addr: bool,
    pub ip_changed: bool,
    pub name: String,
    pub dhcp: bool,
    pub ip: Option<String>,
    pub previous_ip: Option<String>,
    pub netmask: Option<u32>,
    pub gateway: Vec<String>,
    pub dns: Vec<String>,
    /// Whether to enable automatic rollback protection.
    /// Only applicable when is_server_addr=true AND ip_changed=true.
    #[serde(default)]
    pub enable_rollback: Option<bool>,
    /// Whether this change is switching to DHCP (for UI messaging)
    #[serde(default)]
    pub switching_to_dhcp: bool,
}

/// Form data for network configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkFormData {
    pub name: String,
    pub ip_address: String,
    pub dhcp: bool,
    pub prefix_len: u32,
    pub dns: Vec<String>,
    pub gateways: Vec<String>,
}

impl From<&DeviceNetwork> for NetworkFormData {
    fn from(adapter: &DeviceNetwork) -> Self {
        Self {
            name: adapter.name.clone(),
            ip_address: adapter
                .ipv4
                .addrs
                .first()
                .map(|a| a.addr.clone())
                .unwrap_or_default(),
            dhcp: adapter.ipv4.addrs.first().map(|a| a.dhcp).unwrap_or(false),
            prefix_len: adapter
                .ipv4
                .addrs
                .first()
                .map(|a| a.prefix_len)
                .unwrap_or(24),
            dns: adapter.ipv4.dns.clone(),
            gateways: adapter.ipv4.gateways.clone(),
        }
    }
}

/// State of network form (to prevent WebSocket interference)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NetworkFormState {
    #[default]
    Idle,
    Editing {
        adapter_name: String,
        form_data: NetworkFormData,
        original_data: NetworkFormData,
    },
    Submitting {
        adapter_name: String,
        form_data: NetworkFormData,
        original_data: NetworkFormData,
    },
}

impl NetworkFormState {
    /// Transition from Editing to Submitting state
    pub fn to_submitting(&self, target_adapter_name: &str) -> Option<Self> {
        if let Self::Editing {
            adapter_name,
            form_data,
            original_data,
        } = self
        {
            if adapter_name == target_adapter_name {
                Some(Self::Submitting {
                    adapter_name: adapter_name.clone(),
                    form_data: form_data.clone(),
                    original_data: original_data.clone(),
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Transition from Submitting back to Editing state
    pub fn to_editing(&self) -> Option<Self> {
        if let Self::Submitting {
            adapter_name,
            form_data,
            original_data,
        } = self
        {
            Some(Self::Editing {
                adapter_name: adapter_name.clone(),
                form_data: form_data.clone(),
                original_data: original_data.clone(),
            })
        } else {
            None
        }
    }
}

/// State machine for network IP change after configuration.
///
/// This state machine tracks the progress of network configuration changes
/// that affect the device's IP address, including automatic rollback handling.
///
/// # State Machine Diagram
///
/// ```text
///                              ┌─────────────────────────────────────────────────────────┐
///                              │                     START                                │
///                              └────────────────────────┬────────────────────────────────┘
///                                                       │
///                                                       ▼
///                                              ┌────────────────┐
///                              ┌───────────────│      Idle      │───────────────┐
///                              │               └────────────────┘               │
///                              │                       │                        │
///                              │     User applies      │                        │
///                              │    network config     │                        │
///                              │                       ▼                        │
///                              │              ┌────────────────┐                │
///                              │              │ ApplyingConfig │                │
///                              │              └────────────────┘                │
///                              │                       │                        │
///                              │   Backend responds    │                        │
///                              │    successfully       │                        │
///                              │                       ▼                        │
///                              │             ┌─────────────────┐                │
///                              │             │ WaitingForNewIp │                │
///                              │             └─────────────────┘                │
///                              │              │               │                 │
///                              │   Healthcheck│               │Timeout expires  │
///                              │    succeeds  │               │(rollback enabled)│
///                              │              ▼               ▼                 │
///                              │   ┌────────────────┐  ┌─────────────────┐      │
///                              │   │ NewIpReachable │  │ WaitingForOldIp │      │
///                              │   └────────────────┘  └─────────────────┘      │
///                              │          │                    │                │
///                              │   Redirect to │    Healthcheck │                │
///                              │     new IP    │    on old IP   │                │
///                              │          │    │    succeeds    │                │
///                              │          │    │       │        │                │
///                              │          ▼    │       ▼        │                │
///                              │   ┌───────────┴───────────┐    │                │
///                              └───│        SUCCESS        │◄───┘                │
///                                  └───────────────────────┘                     │
///                                                                                │
///                              ┌─────────────────────────────────────────────────┘
///                              │ Timeout expires (rollback disabled)
///                              ▼
///                     ┌────────────────┐
///                     │ NewIpTimeout   │  (Shows manual navigation message)
///                     └────────────────┘
/// ```
///
/// # State Descriptions
///
/// - **Idle**: No network change in progress
/// - **ApplyingConfig**: Configuration request sent to backend, waiting for response
/// - **WaitingForNewIp**: Polling new IP to verify reachability before rollback timeout
/// - **NewIpReachable**: New IP confirmed reachable, will redirect browser
/// - **NewIpTimeout**: Timeout expired without rollback enabled, show manual nav message
/// - **WaitingForOldIp**: Rollback assumed, now polling old IP to verify device is back
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NetworkChangeState {
    /// No network change in progress
    #[default]
    Idle,
    /// Configuration request sent to backend, waiting for response
    ApplyingConfig {
        is_server_addr: bool,
        ip_changed: bool,
        new_ip: String,
        old_ip: String,
        switching_to_dhcp: bool,
    },
    /// Polling new IP to verify reachability before rollback timeout
    WaitingForNewIp {
        new_ip: String,
        old_ip: String,
        attempt: u32,
        rollback_timeout_seconds: u64,
        ui_port: u16,
        switching_to_dhcp: bool,
    },
    /// New IP confirmed reachable, browser will redirect
    NewIpReachable { new_ip: String, ui_port: u16 },
    /// Timeout expired without confirming new IP (rollback disabled case)
    NewIpTimeout {
        new_ip: String,
        old_ip: String,
        ui_port: u16,
        switching_to_dhcp: bool,
    },
    /// Rollback assumed complete, polling old IP to verify device is accessible
    WaitingForOldIp {
        old_ip: String,
        ui_port: u16,
        attempt: u32,
    },
}

/// Response from backend when setting network configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetNetworkConfigResponse {
    pub rollback_timeout_seconds: u64,
    pub ui_port: u16,
    pub rollback_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod validation {
        use super::*;

        #[test]
        fn is_valid_ipv4_accepts_valid_addresses() {
            assert!(is_valid_ipv4("192.168.1.1"));
            assert!(is_valid_ipv4("10.0.0.1"));
            assert!(is_valid_ipv4("172.16.0.1"));
            assert!(is_valid_ipv4("0.0.0.0"));
            assert!(is_valid_ipv4("255.255.255.255"));
        }

        #[test]
        fn is_valid_ipv4_accepts_empty_string() {
            assert!(is_valid_ipv4(""));
        }

        #[test]
        fn is_valid_ipv4_rejects_invalid_addresses() {
            assert!(!is_valid_ipv4("256.1.1.1"));
            assert!(!is_valid_ipv4("192.168.1"));
            assert!(!is_valid_ipv4("192.168.1.1.1"));
            assert!(!is_valid_ipv4("abc.def.ghi.jkl"));
            assert!(!is_valid_ipv4("192.168.-1.1"));
        }

        #[test]
        fn parse_netmask_accepts_valid_values() {
            assert_eq!(parse_netmask("24"), Some(24));
            assert_eq!(parse_netmask("/24"), Some(24));
            assert_eq!(parse_netmask("0"), Some(0));
            assert_eq!(parse_netmask("32"), Some(32));
            assert_eq!(parse_netmask("/8"), Some(8));
        }

        #[test]
        fn parse_netmask_rejects_invalid_values() {
            assert_eq!(parse_netmask("33"), None);
            assert_eq!(parse_netmask("abc"), None);
            assert_eq!(parse_netmask("-1"), None);
            assert_eq!(parse_netmask("24.5"), None);
        }
    }

    mod current_connection {
        use super::*;

        fn create_adapter(name: &str, ip: &str, online: bool) -> DeviceNetwork {
            DeviceNetwork {
                name: name.to_string(),
                mac: "00:11:22:33:44:55".to_string(),
                online,
                file: Some("/etc/network/interfaces".to_string()),
                ipv4: InternetProtocol {
                    addrs: vec![IpAddress {
                        addr: ip.to_string(),
                        dhcp: false,
                        prefix_len: 24,
                    }],
                    dns: vec![],
                    gateways: vec![],
                },
            }
        }

        #[test]
        fn returns_adapter_with_matching_ip() {
            let status = NetworkStatus {
                network_status: vec![
                    create_adapter("eth0", "192.168.1.100", true),
                    create_adapter("eth1", "192.168.2.100", true),
                ],
            };

            let adapter = status.current_connection_adapter(Some("192.168.1.100"));
            assert_eq!(adapter.map(|a| &a.name), Some(&"eth0".to_string()));
        }

        #[test]
        fn returns_none_for_hostname_without_ip_match() {
            let status = NetworkStatus {
                network_status: vec![
                    create_adapter("eth0", "192.168.1.100", false),
                    create_adapter("eth1", "192.168.2.100", true),
                    create_adapter("eth2", "192.168.3.100", true),
                ],
            };

            let adapter = status.current_connection_adapter(Some("omnect-device"));
            assert_eq!(adapter, None);
        }

        #[test]
        fn returns_none_for_no_hostname() {
            let status = NetworkStatus {
                network_status: vec![create_adapter("eth0", "192.168.1.100", true)],
            };

            let adapter = status.current_connection_adapter(None);
            assert_eq!(adapter, None);
        }

        #[test]
        fn returns_none_for_no_match() {
            let status = NetworkStatus {
                network_status: vec![create_adapter("eth0", "192.168.1.100", true)],
            };

            let adapter = status.current_connection_adapter(Some("192.168.99.99"));
            assert_eq!(adapter, None);
        }

        #[test]
        fn returns_none_when_no_online_adapters() {
            let status = NetworkStatus {
                network_status: vec![create_adapter("eth0", "192.168.1.100", false)],
            };

            let adapter = status.current_connection_adapter(Some("omnect-device"));
            assert_eq!(adapter, None);
        }

        #[test]
        fn returns_adapter_with_matching_localhost() {
            let status = NetworkStatus {
                network_status: vec![create_adapter("eth0", "localhost", true)],
            };

            let adapter = status.current_connection_adapter(Some("localhost"));
            assert_eq!(adapter.map(|a| &a.name), Some(&"eth0".to_string()));
        }
    }
}
