use crux_core::Command;

use crate::auth_post;
use crate::events::{DeviceEvent, Event, UiEvent};
use crate::http_get_silent;
use crate::model::Model;
use crate::types::{
    HealthcheckInfo, NetworkChangeState, NetworkConfigRequest, NetworkFormData, NetworkFormState,
    OverlaySpinnerState,
};
use crate::Effect;

/// Success message for network configuration update
const NETWORK_CONFIG_SUCCESS: &str = "Network configuration updated";

/// Handle network configuration request
pub fn handle_set_network_config(config: String, model: &mut Model) -> Command<Effect, Event> {
    // Parse the JSON config to extract metadata
    let parsed_config: Result<NetworkConfigRequest, _> = serde_json::from_str(&config);

    match parsed_config {
        Ok(config_req) => {
            // Store network change state for later use
            // Show modal for: IP changed OR switching to DHCP on current adapter
            if config_req.is_server_addr && (config_req.ip_changed || config_req.switching_to_dhcp)
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
            if let Some(submitting) = model.network_form_state.to_submitting() {
                model.network_form_state = submitting;
            }

            // Clear dirty flag when submitting
            model.network_form_dirty = false;

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

/// Helper to update network state and spinner based on configuration response
fn update_network_state_and_spinner(
    model: &mut Model,
    new_ip: String,
    ui_port: u16,
    rollback_timeout_seconds: u64,
    switching_to_dhcp: bool,
    rollback_enabled: bool,
) {
    // Determine target state
    // If switching to DHCP without rollback, we go to Idle
    if !rollback_enabled && switching_to_dhcp {
        model.network_change_state = NetworkChangeState::Idle;
    } else {
        model.network_change_state = NetworkChangeState::WaitingForNewIp {
            new_ip,
            attempt: 0,
            rollback_timeout_seconds: if rollback_enabled {
                rollback_timeout_seconds
            } else {
                0
            },
            ui_port,
            switching_to_dhcp,
        };
    }

    // Determine overlay text
    let overlay_text = if switching_to_dhcp {
        if rollback_enabled {
            "Network configuration is being applied. Your connection will be interrupted. \
             Use your DHCP server or device console to find the new IP address. \
             You must access the new address to cancel the automatic rollback."
        } else {
            "Network configuration has been applied. Your connection will be interrupted. \
             Use your DHCP server or device console to find the new IP address."
        }
    } else if rollback_enabled {
        "Network configuration is being applied. Click the button below to open the new address in a new tab. \
         You must access the new address to cancel the automatic rollback."
    } else {
        "Network configuration has been applied. Your connection will be interrupted. \
         Click the button below to navigate to the new address."
    };

    let spinner = OverlaySpinnerState::new("Applying network settings").with_text(overlay_text);

    model.overlay_spinner = if rollback_enabled && !switching_to_dhcp {
        spinner.with_countdown(rollback_timeout_seconds as u32)
    } else if rollback_enabled && switching_to_dhcp {
        // Show countdown even for DHCP if rollback is enabled
        spinner.with_countdown(rollback_timeout_seconds as u32)
    } else {
        spinner
    };
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
                switching_to_dhcp,
                ..
            } = &model.network_change_state.clone()
            {
                if response.rollback_enabled {
                    update_network_state_and_spinner(
                        model,
                        new_ip.clone(),
                        response.ui_port,
                        response.rollback_timeout_seconds,
                        *switching_to_dhcp,
                        true,
                    );
                } else {
                    update_network_state_and_spinner(
                        model,
                        new_ip.clone(),
                        response.ui_port,
                        0,
                        *switching_to_dhcp,
                        false,
                    );
                }

                model.success_message = Some(NETWORK_CONFIG_SUCCESS.to_string());
                model.network_form_state = NetworkFormState::Idle;
                crux_core::render::render()
            } else {
                // Not changing current connection's IP - just show success message
                model.success_message = Some(NETWORK_CONFIG_SUCCESS.to_string());
                model.network_change_state = NetworkChangeState::Idle;
                model.network_form_state = NetworkFormState::Idle;
                model.overlay_spinner.clear();
                crux_core::render::render()
            }
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

/// Handle new IP check tick - polls new IP to see if it's reachable
pub fn handle_new_ip_check_tick(model: &mut Model) -> Command<Effect, Event> {
    if let NetworkChangeState::WaitingForNewIp {
        new_ip,
        attempt,
        ui_port,
        switching_to_dhcp,
        ..
    } = &mut model.network_change_state
    {
        *attempt += 1;

        // If switching to DHCP, we don't know the new IP, so we can't poll it.
        // We just wait for the timeout (rollback) or for the user to manually navigate.
        if !*switching_to_dhcp {
            // Try to reach the new IP (silent GET - no error shown on failure)
            // Use HTTPS since the server only listens on HTTPS
            let url = format!("https://{new_ip}:{ui_port}/healthcheck");
            http_get_silent!(
                url,
                on_success: Event::Device(DeviceEvent::HealthcheckResponse(Ok(
                    HealthcheckInfo::default()
                ))),
                on_error: Event::Ui(UiEvent::ClearSuccess)
            )
        } else {
            crux_core::render::render()
        }
    } else {
        crux_core::render::render()
    }
}

/// Handle new IP check timeout - new IP didn't become reachable in time
pub fn handle_new_ip_check_timeout(model: &mut Model) -> Command<Effect, Event> {
    if let NetworkChangeState::WaitingForNewIp {
        new_ip, ui_port, ..
    } = &model.network_change_state
    {
        let new_ip_url = format!("https://{new_ip}:{ui_port}");
        model.network_change_state = NetworkChangeState::NewIpTimeout {
            new_ip: new_ip.clone(),
            ui_port: *ui_port,
        };

        // Update overlay spinner to show timeout with manual link
        model.overlay_spinner.set_text(
            format!(
                "Automatic rollback will occur soon. The network settings were not confirmed at the new address. \
                 Please navigate to: {new_ip_url}"
            )
            .as_str(),
        );
        model.overlay_spinner.set_timed_out();
    }

    crux_core::render::render()
}

/// Handle network form start edit - initialize form with current network adapter data
pub fn handle_network_form_start_edit(
    adapter_name: String,
    model: &mut Model,
) -> Command<Effect, Event> {
    // Find the network adapter and copy its data to form state
    if let Some(network_status) = &model.network_status {
        if let Some(adapter) = network_status
            .network_status
            .iter()
            .find(|n| n.name == adapter_name)
        {
            let form_data = NetworkFormData::from(adapter);

            model.network_form_state = NetworkFormState::Editing {
                adapter_name: adapter_name.clone(),
                form_data: form_data.clone(),
                original_data: form_data,
            };
            // Clear dirty flag when starting a fresh edit
            model.network_form_dirty = false;
        }
    }

    crux_core::render::render()
}

/// Handle network form update - update form data from user input
pub fn handle_network_form_update(
    form_data_json: String,
    model: &mut Model,
) -> Command<Effect, Event> {
    // Parse the JSON form data
    let parsed: Result<NetworkFormData, _> = serde_json::from_str(&form_data_json);

    match parsed {
        Ok(form_data) => {
            if let NetworkFormState::Editing {
                adapter_name,
                original_data,
                ..
            } = &model.network_form_state
            {
                let is_dirty = form_data != *original_data;

                model.network_form_state = NetworkFormState::Editing {
                    adapter_name: adapter_name.clone(),
                    form_data,
                    original_data: original_data.clone(),
                };
                model.network_form_dirty = is_dirty;
            }
            crux_core::render::render()
        }
        Err(e) => model.set_error_and_render(format!("Invalid form data: {e}")),
    }
}

/// Handle acknowledge network rollback - clear the rollback occurred flag
pub fn handle_ack_rollback(model: &mut Model) -> Command<Effect, Event> {
    // Clear the rollback status in the model
    if let Some(healthcheck) = &mut model.healthcheck {
        healthcheck.network_rollback_occurred = false;
    }

    // Send POST request to backend to clear the marker file
    auth_post!(
        Device,
        DeviceEvent,
        model,
        "/ack-rollback",
        AckRollbackResponse,
        "Acknowledge rollback"
    )
}
