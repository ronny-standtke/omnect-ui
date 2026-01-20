use crux_core::Command;

use crate::{
    events::{Event, WebSocketEvent},
    model::Model,
    update_field, CentrifugoCmd, Effect,
};

/// Handle WebSocket and Centrifugo-related events
pub fn handle(event: WebSocketEvent, model: &mut Model) -> Command<Effect, Event> {
    match event {
        WebSocketEvent::SubscribeToChannels => {
            // Issue Centrifugo effect (shell sends WebSocket data as events directly)
            CentrifugoCmd::subscribe_all()
                .build()
                .then_send(|_| Event::WebSocket(WebSocketEvent::Connected))
        }

        WebSocketEvent::UnsubscribeFromChannels => {
            // Issue Centrifugo effect
            CentrifugoCmd::unsubscribe_all()
                .build()
                .then_send(|_| Event::WebSocket(WebSocketEvent::Disconnected))
        }

        WebSocketEvent::SystemInfoUpdated(info) => update_field!(model.system_info, Some(info)),
        WebSocketEvent::NetworkStatusUpdated(status) => {
            model.network_status = Some(status);
            model.update_current_connection_adapter();
            crux_core::render::render()
        }
        WebSocketEvent::OnlineStatusUpdated(status) => {
            update_field!(model.online_status, Some(status))
        }
        WebSocketEvent::FactoryResetUpdated(reset) => {
            update_field!(model.factory_reset, Some(reset))
        }
        WebSocketEvent::UpdateValidationStatusUpdated(status) => {
            update_field!(model.update_validation_status, Some(status))
        }
        WebSocketEvent::TimeoutsUpdated(timeouts) => update_field!(model.timeouts, Some(timeouts)),
        WebSocketEvent::Connected => update_field!(model.is_connected, true),
        WebSocketEvent::Disconnected => update_field!(model.is_connected, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FactoryReset, OsInfo, SystemInfo, Timeouts, UpdateValidationStatus};
    use crate::OnlineStatus;

    mod system_info {
        use super::*;

        #[test]
        fn updates_system_info() {
            let mut model = Model::default();

            let info = SystemInfo {
                os: OsInfo {
                    name: "Linux".into(),
                    version: "5.10".into(),
                },
                azure_sdk_version: "1.0".into(),
                omnect_device_service_version: "2.0".into(),
                boot_time: Some("2024-01-01".into()),
            };

            let _ = handle(WebSocketEvent::SystemInfoUpdated(info.clone()), &mut model);

            assert_eq!(model.system_info, Some(info));
        }

        #[test]
        fn replaces_previous_system_info() {
            let old_info = SystemInfo {
                os: OsInfo {
                    name: "Linux".into(),
                    version: "5.9".into(),
                },
                azure_sdk_version: "0.9".into(),
                omnect_device_service_version: "1.9".into(),
                boot_time: None,
            };
            let mut model = Model {
                system_info: Some(old_info),
                ..Default::default()
            };

            let new_info = SystemInfo {
                os: OsInfo {
                    name: "Linux".into(),
                    version: "5.10".into(),
                },
                azure_sdk_version: "1.0".into(),
                omnect_device_service_version: "2.0".into(),
                boot_time: Some("2024-01-01".into()),
            };

            let _ = handle(
                WebSocketEvent::SystemInfoUpdated(new_info.clone()),
                &mut model,
            );

            assert_eq!(model.system_info, Some(new_info));
        }
    }

    mod online_status {
        use super::*;

        #[test]
        fn updates_online_status_to_online() {
            let mut model = Model::default();

            let _ = handle(
                WebSocketEvent::OnlineStatusUpdated(OnlineStatus { iothub: true }),
                &mut model,
            );

            assert_eq!(model.online_status, Some(OnlineStatus { iothub: true }));
        }

        #[test]
        fn updates_online_status_to_offline() {
            let mut model = Model {
                online_status: Some(OnlineStatus { iothub: true }),
                ..Default::default()
            };

            let _ = handle(
                WebSocketEvent::OnlineStatusUpdated(OnlineStatus { iothub: false }),
                &mut model,
            );

            assert_eq!(model.online_status, Some(OnlineStatus { iothub: false }));
        }

        #[test]
        fn transitions_from_offline_to_online() {
            let mut model = Model {
                online_status: Some(OnlineStatus { iothub: false }),
                ..Default::default()
            };

            let _ = handle(
                WebSocketEvent::OnlineStatusUpdated(OnlineStatus { iothub: true }),
                &mut model,
            );

            assert_eq!(model.online_status, Some(OnlineStatus { iothub: true }));
        }
    }

    mod factory_reset {
        use super::*;

        #[test]
        fn updates_factory_reset_status() {
            let mut model = Model::default();

            let status = FactoryReset {
                keys: vec!["test_key".into()],
                result: None,
            };

            let _ = handle(
                WebSocketEvent::FactoryResetUpdated(status.clone()),
                &mut model,
            );

            assert_eq!(model.factory_reset, Some(status));
        }
    }

    mod update_validation {
        use super::*;

        #[test]
        fn updates_validation_status() {
            let mut model = Model::default();

            let status = UpdateValidationStatus {
                status: "Succeeded".into(),
            };

            let _ = handle(
                WebSocketEvent::UpdateValidationStatusUpdated(status.clone()),
                &mut model,
            );

            assert_eq!(model.update_validation_status, Some(status));
        }
    }

    mod timeouts {
        use super::*;
        use crate::types::Duration;

        #[test]
        fn updates_timeouts() {
            let mut model = Model::default();

            let timeouts = Timeouts {
                wait_online_timeout: Duration {
                    nanos: 0,
                    secs: 300,
                },
            };

            let _ = handle(
                WebSocketEvent::TimeoutsUpdated(timeouts.clone()),
                &mut model,
            );

            assert_eq!(model.timeouts, Some(timeouts));
        }
    }

    mod connection {
        use super::*;

        #[test]
        fn connected_sets_is_connected() {
            let mut model = Model::default();

            let _ = handle(WebSocketEvent::Connected, &mut model);

            assert!(model.is_connected);
        }

        #[test]
        fn disconnected_clears_is_connected() {
            let mut model = Model {
                is_connected: true,
                ..Default::default()
            };

            let _ = handle(WebSocketEvent::Disconnected, &mut model);

            assert!(!model.is_connected);
        }
    }

    mod network_status {
        use super::*;
        use crate::types::{DeviceNetwork, InternetProtocol, IpAddress, NetworkStatus};

        #[test]
        fn updates_network_status() {
            let mut model = Model::default();

            let status = NetworkStatus {
                network_status: vec![DeviceNetwork {
                    name: "eth0".to_string(),
                    mac: "00:11:22:33:44:55".to_string(),
                    online: true,
                    file: Some("/etc/network/interfaces".to_string()),
                    ipv4: InternetProtocol {
                        addrs: vec![IpAddress {
                            addr: "192.168.1.100".to_string(),
                            dhcp: false,
                            prefix_len: 24,
                        }],
                        dns: vec![],
                        gateways: vec![],
                    },
                }],
            };

            let _ = handle(
                WebSocketEvent::NetworkStatusUpdated(status.clone()),
                &mut model,
            );

            assert_eq!(model.network_status, Some(status));
        }

        #[test]
        fn updates_current_connection_adapter_when_browser_hostname_set() {
            let mut model = Model {
                browser_hostname: Some("192.168.1.100".to_string()),
                ..Default::default()
            };

            let status = NetworkStatus {
                network_status: vec![DeviceNetwork {
                    name: "eth0".to_string(),
                    mac: "00:11:22:33:44:55".to_string(),
                    online: true,
                    file: Some("/etc/network/interfaces".to_string()),
                    ipv4: InternetProtocol {
                        addrs: vec![IpAddress {
                            addr: "192.168.1.100".to_string(),
                            dhcp: false,
                            prefix_len: 24,
                        }],
                        dns: vec![],
                        gateways: vec![],
                    },
                }],
            };

            let _ = handle(WebSocketEvent::NetworkStatusUpdated(status), &mut model);

            assert_eq!(model.current_connection_adapter, Some("eth0".to_string()));
        }
    }
}
