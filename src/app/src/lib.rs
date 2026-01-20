pub mod commands;
pub mod events;
pub mod http_helpers;
pub mod macros;
pub mod model;
pub mod types;
pub mod update;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

use crux_core::Command;

// Re-export core types
pub use crate::{
    commands::centrifugo::{CentrifugoOperation, CentrifugoOutput},
    events::Event,
    http_helpers::{
        build_url, check_response_status, extract_error_message, extract_string_response,
        handle_auth_error, handle_request_error, is_response_success, parse_json_response,
        process_json_response, process_status_response, BASE_URL,
    },
    model::Model,
    types::*,
};
pub use crux_http::Result as HttpResult;

#[crux_macros::effect(typegen)]
pub enum Effect {
    Render(crux_core::render::RenderOperation),
    Http(crux_http::protocol::HttpRequest),
    Centrifugo(CentrifugoOperation),
}

pub type CentrifugoCmd = crate::commands::centrifugo::Centrifugo<Effect, Event>;
pub type HttpCmd = crux_http::command::Http<Effect, Event>;

/// The Core application
#[derive(Default)]
pub struct App;

impl crux_core::App for App {
    type Event = Event;
    type Model = Model;
    type ViewModel = Model;
    type Effect = Effect;

    fn update(&self, event: Self::Event, model: &mut Self::Model) -> Command<Effect, Event> {
        update::update(event, model)
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        model.clone()
    }
}
