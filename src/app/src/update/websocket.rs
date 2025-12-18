use crux_core::Command;

use crate::events::{Event, WebSocketEvent};
use crate::model::Model;
use crate::update_field;
use crate::{CentrifugoCmd, Effect};

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
            update_field!(model.network_status, Some(status))
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
