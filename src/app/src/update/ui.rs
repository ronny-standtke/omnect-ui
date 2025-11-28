use crux_core::Command;

use crate::events::Event;
use crate::model::Model;
use crate::update_field;
use crate::Effect;

/// Handle UI-related events (clear messages, etc.)
pub fn handle(event: Event, model: &mut Model) -> Command<Effect, Event> {
    match event {
        Event::ClearError => update_field!(model.error_message, None),
        Event::ClearSuccess => update_field!(model.success_message, None),
        _ => unreachable!("Non-UI event passed to UI handler"),
    }
}
