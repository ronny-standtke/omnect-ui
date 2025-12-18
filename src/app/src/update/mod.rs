mod auth;
mod device;
mod ui;
mod websocket;

use crux_core::{render::render, Command};

use crate::events::Event;
use crate::model::Model;
use crate::Effect;

/// Main update dispatcher - routes events to domain-specific handlers
pub fn update(event: Event, model: &mut Model) -> Command<Effect, Event> {
    // Log to browser console for debugging
    log::debug!("Crux Core update: {event:?}");

    match event {
        Event::Initialize => {
            model.start_loading();
            render()
        }
        Event::Auth(auth_event) => auth::handle(auth_event, model),
        Event::Device(device_event) => device::handle(device_event, model),
        Event::WebSocket(ws_event) => websocket::handle(ws_event, model),
        Event::Ui(ui_event) => ui::handle(ui_event, model),
    }
}
