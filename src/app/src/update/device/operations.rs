use crux_core::Command;

use crate::model::Model;
use crate::types::{DeviceOperationState, OverlaySpinnerState};
use crate::Effect;

pub const REBOOT_TIMEOUT_SECS: u32 = 300; // 5 minutes
pub const FACTORY_RESET_TIMEOUT_SECS: u32 = 600; // 10 minutes

/// Check if an error message indicates a network error
pub fn is_network_error(error: &str) -> bool {
    let e_lower = error.to_lowercase();
    e_lower.contains("failed to fetch")
        || e_lower.contains("networkerror")
        || error.contains("IO error")
}

/// Check if an update is complete based on healthcheck status
pub fn is_update_complete(info: &crate::types::HealthcheckInfo) -> bool {
    let status = &info.update_validation_status.status;
    status == "Succeeded" || status == "Recovered" || status == "NoUpdate"
}

/// Generic handler for device operation responses (reboot, factory reset, update)
pub fn handle_device_operation_response(
    result: Result<(), String>,
    model: &mut Model,
    operation: DeviceOperationState,
    success_msg: &str,
    connection_lost_msg: &str,
    overlay_title: &str,
    overlay_text: Option<String>,
) -> Command<Effect, crate::Event> {
    model.stop_loading();

    let is_network_err = result.as_ref().is_err_and(|e| is_network_error(e));

    if result.is_ok() || is_network_err {
        model.success_message = Some(if is_network_err {
            connection_lost_msg.to_string()
        } else {
            success_msg.to_string()
        });
        let timeout_secs = match &operation {
            DeviceOperationState::FactoryResetting => FACTORY_RESET_TIMEOUT_SECS,
            _ => REBOOT_TIMEOUT_SECS,
        };
        model.device_operation_state = operation;
        model.reconnection_attempt = 0;
        model.device_went_offline = false;
        let mut spinner = OverlaySpinnerState::new(overlay_title).with_countdown(timeout_secs);
        if let Some(text) = overlay_text {
            spinner = spinner.with_text(text);
        }
        model.overlay_spinner = spinner;
    } else if let Err(e) = result {
        model.set_error(e);
        model.overlay_spinner.clear();
    }

    crux_core::render::render()
}
