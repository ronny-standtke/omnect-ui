use crux_core::Command;

use crate::auth_post;
use crate::events::Event;
use crate::handle_response;
use crate::model::Model;
use crate::types::{FactoryResetRequest, LoadUpdateRequest, RunUpdateRequest};
use crate::Effect;

/// Handle device action events (reboot, factory reset, network, updates)
pub fn handle(event: Event, model: &mut Model) -> Command<Effect, Event> {
    match event {
        Event::Reboot => auth_post!(model, "/api/device/reboot", RebootResponse, "Reboot"),

        Event::RebootResponse(result) => handle_response!(model, result, {
            success_message: "Reboot initiated",
        }),

        Event::FactoryResetRequest { mode, preserve } => {
            let request = FactoryResetRequest { mode, preserve };
            auth_post!(model, "/api/device/factory-reset", FactoryResetResponse, "Factory reset",
                body_json: &request
            )
        }

        Event::FactoryResetResponse(result) => handle_response!(model, result, {
            success_message: "Factory reset initiated",
        }),

        Event::ReloadNetwork => {
            auth_post!(
                model,
                "/api/device/reload-network",
                ReloadNetworkResponse,
                "Reload network"
            )
        }

        Event::ReloadNetworkResponse(result) => handle_response!(model, result, {
            success_message: "Network reloaded",
        }),

        Event::SetNetworkConfig { config } => {
            auth_post!(model, "/api/device/network", SetNetworkConfigResponse, "Set network config",
                body_string: config
            )
        }

        Event::SetNetworkConfigResponse(result) => handle_response!(model, result, {
            success_message: "Network configuration updated",
        }),

        Event::LoadUpdate { file_path } => {
            let request = LoadUpdateRequest { file_path };
            auth_post!(model, "/api/update/load", LoadUpdateResponse, "Load update",
                body_json: &request
            )
        }

        Event::LoadUpdateResponse(result) => handle_response!(model, result, {
            success_message: "Update loaded",
        }),

        Event::RunUpdate { validate_iothub } => {
            let request = RunUpdateRequest { validate_iothub };
            auth_post!(model, "/api/update/run", RunUpdateResponse, "Run update",
                body_json: &request
            )
        }

        Event::RunUpdateResponse(result) => handle_response!(model, result, {
            success_message: "Update started",
        }),

        Event::HealthcheckResponse(result) => handle_response!(model, result, {
            on_success: |model, info| {
                model.healthcheck = Some(info);
            },
            no_loading: true,
        }),

        _ => unreachable!("Non-device event passed to device handler"),
    }
}
