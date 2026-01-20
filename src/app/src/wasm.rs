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

/// Initialize the WASM module and set up logging
///
/// This runs automatically when the WASM module is loaded.
/// To control log level from JavaScript:
/// ```javascript
/// // Set log level before importing WASM module
/// localStorage.setItem('rust_log', 'debug'); // or 'info', 'warn', 'error'
/// ```
#[wasm_bindgen(start)]
pub fn init_wasm() {
    // Initialize console_log with log level from localStorage or default to Info
    console_log::init_with_level(log::Level::Debug).expect("Failed to initialize logger");
}

/// Process an event from JavaScript
///
/// Takes a bincode-serialized Event and returns bincode-serialized Effects.
#[wasm_bindgen]
pub fn process_event(event_bytes: &[u8]) -> Vec<u8> {
    let mut effects = Vec::new();
    CORE.update(event_bytes, &mut effects)
        .expect("Failed to process event");
    effects
}

/// Get the current view model
///
/// Returns a bincode-serialized ViewModel.
#[wasm_bindgen]
pub fn view() -> Vec<u8> {
    let mut view = Vec::new();
    CORE.view(&mut view).expect("Failed to get view model");
    view
}

/// Handle a response to an effect
///
/// Takes an effect ID and bincode-serialized response data.
/// Returns bincode-serialized Effects that should be processed.
#[wasm_bindgen]
pub fn handle_response(id: u32, response_bytes: &[u8]) -> Vec<u8> {
    let mut effects = Vec::new();
    CORE.resolve(
        crux_core::bridge::EffectId(id),
        response_bytes,
        &mut effects,
    )
    .expect("Failed to handle response");
    effects
}
