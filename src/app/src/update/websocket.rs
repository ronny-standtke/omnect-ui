use crux_core::{render::render, Command};

use crate::capabilities::centrifugo::CentrifugoOutput;
use crate::events::Event;
use crate::model::Model;
use crate::parse_channel_data;
use crate::types::*;
use crate::update_field;
use crate::{CentrifugoCmd, Effect};

/// Handle WebSocket and Centrifugo-related events
pub fn handle(event: Event, model: &mut Model) -> Command<Effect, Event> {
    match event {
        Event::SubscribeToChannels => CentrifugoCmd::subscribe_all()
            .build()
            .then_send(Event::CentrifugoResponse),

        Event::UnsubscribeFromChannels => CentrifugoCmd::unsubscribe_all()
            .build()
            .then_send(Event::CentrifugoResponse),

        Event::CentrifugoResponse(output) => handle_centrifugo_output(output, model),

        Event::SystemInfoUpdated(info) => update_field!(model.system_info, Some(info)),
        Event::NetworkStatusUpdated(status) => update_field!(model.network_status, Some(status)),
        Event::OnlineStatusUpdated(status) => update_field!(model.online_status, Some(status)),
        Event::FactoryResetUpdated(reset) => update_field!(model.factory_reset, Some(reset)),
        Event::UpdateValidationStatusUpdated(status) => {
            update_field!(model.update_validation_status, Some(status))
        }
        Event::TimeoutsUpdated(timeouts) => update_field!(model.timeouts, Some(timeouts)),
        Event::Connected => update_field!(model.is_connected, true),
        Event::Disconnected => update_field!(model.is_connected, false),

        _ => unreachable!("Non-websocket event passed to websocket handler"),
    }
}

/// Handle Centrifugo output messages
fn handle_centrifugo_output(output: CentrifugoOutput, model: &mut Model) -> Command<Effect, Event> {
    match output {
        CentrifugoOutput::Connected => {
            model.is_connected = true;
            render()
        }
        CentrifugoOutput::Disconnected => {
            model.is_connected = false;
            render()
        }
        CentrifugoOutput::Subscribed { channel: _ } => render(),
        CentrifugoOutput::Unsubscribed { channel: _ } => render(),
        CentrifugoOutput::Message { channel, data } => {
            parse_channel_message(&channel, &data, model);
            render()
        }
        CentrifugoOutput::HistoryResult { channel, data } => {
            if let Some(json_data) = data {
                parse_channel_message(&channel, &json_data, model);
            }
            render()
        }
        CentrifugoOutput::Error { message } => {
            model.error_message = Some(format!("Centrifugo error: {message}"));
            render()
        }
    }
}

/// Parse JSON data from a channel message and update the model
fn parse_channel_message(channel: &str, data: &str, model: &mut Model) {
    parse_channel_data! {
        channel, data, model,
        "SystemInfoV1" => system_info: SystemInfo,
        "NetworkStatusV1" => network_status: NetworkStatus,
        "OnlineStatusV1" => online_status: OnlineStatus,
        "FactoryResetV1" => factory_reset: FactoryReset,
        "UpdateValidationStatusV1" => update_validation_status: UpdateValidationStatus,
        "TimeoutsV1" => timeouts: Timeouts,
    }
}
