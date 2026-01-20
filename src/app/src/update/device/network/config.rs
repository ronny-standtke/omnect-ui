use crux_core::Command;

use crate::{
    auth_post,
    events::Event,
    model::Model,
    types::{NetworkChangeState, NetworkConfigRequest, NetworkFormState},
    Effect,
};

use super::verification::update_network_state_and_spinner;

/// Success message for network configuration update
const NETWORK_CONFIG_SUCCESS: &str = "Network configuration updated";

/// Handle network configuration request
pub fn handle_set_network_config(config: String, model: &mut Model) -> Command<Effect, Event> {
    // Parse the JSON config to extract metadata
    let parsed_config: Result<NetworkConfigRequest, _> = serde_json::from_str(&config);

    match parsed_config {
        Ok(config_req) => {
            let is_server_addr = model.is_current_adapter(&config_req.name);

            // Store network change state for later use
            // Show modal for: current connection AND (IP changed OR switching to DHCP OR rollback explicitly enabled)
            if is_server_addr
                && (config_req.ip_changed
                    || config_req.switching_to_dhcp
                    || config_req.enable_rollback.unwrap_or(false))
            {
                model.network_change_state = NetworkChangeState::ApplyingConfig {
                    is_server_addr: true,
                    ip_changed: config_req.ip_changed || config_req.switching_to_dhcp,
                    new_ip: config_req.ip.clone().unwrap_or_default(),
                    old_ip: config_req.previous_ip.clone().unwrap_or_default(),
                    switching_to_dhcp: config_req.switching_to_dhcp,
                };
            }

            // Transition network form to submitting state
            if let Some(submitting) = model.network_form_state.to_submitting(&config_req.name) {
                model.network_form_state = submitting;
            }

            // Clear dirty flag when submitting
            model.network_form_dirty = false;

            // Clear any previous messages so that identical subsequent messages
            // (e.g. from multiple network config applies) trigger the UI watcher correctly.
            model.success_message = None;
            model.error_message = None;

            // Send the request to backend
            auth_post!(
                Device,
                DeviceEvent,
                model,
                "/network",
                SetNetworkConfigResponse,
                "Set network config",
                body_string: config,
                expect_json: crate::types::SetNetworkConfigResponse
            )
        }
        Err(e) => model.set_error_and_render(format!("Invalid network config: {e}")),
    }
}

/// Handle network configuration response
pub fn handle_set_network_config_response(
    result: Result<crate::types::SetNetworkConfigResponse, String>,
    model: &mut Model,
) -> Command<Effect, Event> {
    model.stop_loading();

    match result {
        Ok(response) => {
            // Check if we are applying a config that changes IP/DHCP
            if let NetworkChangeState::ApplyingConfig {
                new_ip,
                old_ip,
                switching_to_dhcp,
                ..
            } = &model.network_change_state.clone()
            {
                if response.rollback_enabled {
                    update_network_state_and_spinner(
                        model,
                        new_ip.clone(),
                        old_ip.clone(),
                        response.ui_port,
                        response.rollback_timeout_seconds,
                        *switching_to_dhcp,
                        true,
                    );
                } else {
                    update_network_state_and_spinner(
                        model,
                        new_ip.clone(),
                        old_ip.clone(),
                        response.ui_port,
                        0,
                        *switching_to_dhcp,
                        false,
                    );
                }
            } else {
                // Not changing current connection's IP - just clear state
                model.network_change_state = NetworkChangeState::Idle;
                model.overlay_spinner.clear();
            }

            model.success_message = Some(NETWORK_CONFIG_SUCCESS.to_string());

            // Transition back to editing state with the new data as original
            if let NetworkFormState::Submitting {
                adapter_name,
                form_data,
                ..
            } = &model.network_form_state
            {
                model.network_form_state = NetworkFormState::Editing {
                    adapter_name: adapter_name.clone(),
                    original_data: form_data.clone(),
                    form_data: form_data.clone(),
                };
            } else {
                model.network_form_state = NetworkFormState::Idle;
            }

            // Clear rollback modal flag after config is applied
            model.should_show_rollback_modal = false;
            crux_core::render::render()
        }
        Err(e) => {
            model.set_error(e);
            model.network_change_state = NetworkChangeState::Idle;
            // Reset form state back to editing on failure
            if let Some(editing) = model.network_form_state.to_editing() {
                model.network_form_state = editing;
            }
            crux_core::render::render()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{NetworkChangeState, NetworkFormState};

    #[test]
    fn static_ip_with_rollback_enters_waiting_state() {
        let mut model = Model {
            current_connection_adapter: Some("eth0".to_string()),
            ..Default::default()
        };
        let config = r#"{
            "isServerAddr": true,
            "ipChanged": true,
            "name": "eth0",
            "dhcp": false,
            "ip": "192.168.1.100",
            "netmask": 24,
            "gateway": [],
            "dns": [],
            "enableRollback": true,
            "switchingToDhcp": false
        }"#
        .to_string();

        let _ = handle_set_network_config(config, &mut model);

        assert!(matches!(
            model.network_change_state,
            NetworkChangeState::ApplyingConfig { .. }
        ));
    }

    #[test]
    fn static_ip_without_rollback_enters_waiting_state() {
        let mut model = Model {
            current_connection_adapter: Some("eth0".to_string()),
            ..Default::default()
        };
        let config = r#"{
            "isServerAddr": true,
            "ipChanged": true,
            "name": "eth0",
            "dhcp": false,
            "ip": "192.168.1.100",
            "netmask": 24,
            "gateway": [],
            "dns": [],
            "enableRollback": false,
            "switchingToDhcp": false
        }"#
        .to_string();

        let _ = handle_set_network_config(config, &mut model);

        assert!(matches!(
            model.network_change_state,
            NetworkChangeState::ApplyingConfig { .. }
        ));
    }

    #[test]
    fn dhcp_with_rollback_enters_waiting_state() {
        let mut model = Model {
            current_connection_adapter: Some("eth0".to_string()),
            ..Default::default()
        };
        let config = r#"{
            "isServerAddr": true,
            "ipChanged": true,
            "name": "eth0",
            "dhcp": true,
            "gateway": [],
            "dns": [],
            "enableRollback": true,
            "switchingToDhcp": true
        }"#
        .to_string();

        let _ = handle_set_network_config(config, &mut model);

        assert!(matches!(
            model.network_change_state,
            NetworkChangeState::ApplyingConfig { .. }
        ));
    }

    #[test]
    fn dhcp_without_rollback_goes_to_idle() {
        let mut model = Model {
            current_connection_adapter: Some("eth0".to_string()),
            ..Default::default()
        };
        let config =
            r#"{"name": "eth0", "dhcp": true, "enableRollback": false, "switchingToDhcp": true}"#
                .to_string();

        let _ = handle_set_network_config(config, &mut model);

        assert!(matches!(
            model.network_change_state,
            NetworkChangeState::Idle
        ));
    }

    #[test]
    fn non_server_adapter_returns_to_idle() {
        let mut model = Model {
            network_form_state: NetworkFormState::Submitting {
                adapter_name: "wlan0".to_string(),
                form_data: crate::types::NetworkFormData {
                    name: "wlan0".to_string(),
                    ip_address: "192.168.1.100".to_string(),
                    dhcp: false,
                    prefix_len: 24,
                    dns: vec![],
                    gateways: vec![],
                },
                original_data: crate::types::NetworkFormData {
                    name: "wlan0".to_string(),
                    ip_address: "192.168.1.100".to_string(),
                    dhcp: false,
                    prefix_len: 24,
                    dns: vec![],
                    gateways: vec![],
                },
            },
            ..Default::default()
        };

        let result = Ok(crate::types::SetNetworkConfigResponse {
            rollback_timeout_seconds: 0,
            ui_port: 80,
            rollback_enabled: false,
        });

        let _ = handle_set_network_config_response(result, &mut model);

        assert!(matches!(
            model.network_form_state,
            NetworkFormState::Editing { .. }
        ));
    }

    #[test]
    fn non_server_adapter_returns_to_editing_state() {
        let mut model = Model {
            network_form_state: NetworkFormState::Submitting {
                adapter_name: "wlan0".to_string(),
                form_data: crate::types::NetworkFormData {
                    name: "wlan0".to_string(),
                    ip_address: "192.168.1.100".to_string(),
                    dhcp: false,
                    prefix_len: 24,
                    dns: vec![],
                    gateways: vec![],
                },
                original_data: crate::types::NetworkFormData {
                    name: "wlan0".to_string(),
                    ip_address: "192.168.1.100".to_string(),
                    dhcp: false,
                    prefix_len: 24,
                    dns: vec![],
                    gateways: vec![],
                },
            },
            ..Default::default()
        };

        let result = Ok(crate::types::SetNetworkConfigResponse {
            rollback_timeout_seconds: 0,
            ui_port: 80,
            rollback_enabled: false,
        });

        let _ = handle_set_network_config_response(result, &mut model);

        assert!(matches!(
            model.network_form_state,
            NetworkFormState::Editing { .. }
        ));
    }

    #[test]
    fn error_resets_to_editing_state() {
        let mut model = Model {
            network_form_state: NetworkFormState::Submitting {
                adapter_name: "eth0".to_string(),
                form_data: crate::types::NetworkFormData {
                    name: "eth0".to_string(),
                    ip_address: "192.168.1.100".to_string(),
                    dhcp: false,
                    prefix_len: 24,
                    dns: vec![],
                    gateways: vec![],
                },
                original_data: crate::types::NetworkFormData {
                    name: "eth0".to_string(),
                    ip_address: "192.168.1.100".to_string(),
                    dhcp: false,
                    prefix_len: 24,
                    dns: vec![],
                    gateways: vec![],
                },
            },
            ..Default::default()
        };

        let result = Err("Failed to set config".to_string());

        let _ = handle_set_network_config_response(result, &mut model);

        assert!(matches!(
            model.network_form_state,
            NetworkFormState::Editing { .. }
        ));
        assert_eq!(model.error_message, Some("Failed to set config".into()));
    }
}
