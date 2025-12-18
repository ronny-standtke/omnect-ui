use crux_core::Command;

use crate::events::{Event, UiEvent};
use crate::model::Model;
use crate::update_field;
use crate::Effect;

/// Handle UI-related events (clear messages, etc.)
pub fn handle(event: UiEvent, model: &mut Model) -> Command<Effect, Event> {
    match event {
        UiEvent::ClearError => update_field!(model.error_message, None),
        UiEvent::ClearSuccess => update_field!(model.success_message, None),
    }
}
