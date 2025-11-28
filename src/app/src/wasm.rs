//! WebAssembly FFI bindings for the Crux Core
//!
//! This module provides the interface between JavaScript and the Crux Core.
//! It exposes functions for processing events and retrieving the view model.

use lazy_static::lazy_static;
use wasm_bindgen::prelude::wasm_bindgen;

use crux_core::{bridge::Bridge, Core};

use crate::App;

lazy_static! {
    static ref CORE: Bridge<App> = Bridge::new(Core::new());
}

/// Process an event from JavaScript
///
/// Takes a bincode-serialized Event and returns bincode-serialized Effects.
#[wasm_bindgen]
pub fn process_event(event_bytes: &[u8]) -> Vec<u8> {
    CORE.process_event(event_bytes)
        .expect("Failed to process event")
}

/// Get the current view model
///
/// Returns a bincode-serialized ViewModel.
#[wasm_bindgen]
pub fn view() -> Vec<u8> {
    CORE.view().expect("Failed to get view model")
}

/// Handle a response to an effect
///
/// Takes an effect ID and bincode-serialized response data.
/// Returns bincode-serialized Effects that should be processed.
#[wasm_bindgen]
pub fn handle_response(id: u32, response_bytes: &[u8]) -> Vec<u8> {
    CORE.handle_response(id, response_bytes)
        .expect("Failed to handle response")
}
