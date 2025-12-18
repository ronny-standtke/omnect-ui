//! HTTP helper functions for Crux Core
//!
//! This module extracts common HTTP response handling logic from macros
//! into debuggable, testable functions.
//!
//! ## Shell Workaround
//!
//! The shell uses an `x-original-status` header to preserve error status codes.
//! This is needed because `crux_http` (v0.15) discards response bodies for 4xx/5xx
//! status codes. The shell masks these as 200 OK and passes the original status
//! in this header, allowing the Core to properly extract error messages.

use crux_http::Response;

/// Base URL for omnect-device API endpoints.
///
/// NOTE: This URL is prefixed as a workaround because `crux_http` (v0.15) panics
/// when given a relative URL in some environments (e.g. `cargo test`).
/// The UI shell (`useCore.ts`) strips this prefix before sending the request.
pub const BASE_URL: &str = "http://omnect-device";

/// Constructs the full address from a given endpoint.
///
/// # Arguments
/// * `endpoint` - The API endpoint path (e.g., "/api/device/reboot")
///
/// # Returns
/// A string containing the full URL: `http://omnect-device{endpoint}`
///
/// # Example
/// ```
/// use omnect_ui_core::http_helpers::build_url;
/// let url = build_url("/api/device/reboot");
/// assert_eq!(url, "http://omnect-device/api/device/reboot");
/// ```
pub fn build_url(endpoint: &str) -> String {
    format!("{BASE_URL}{endpoint}")
}

/// Validates HTTP response, accounting for shell workaround.
///
/// Returns `true` if the response status is 2xx AND there is no
/// `x-original-status` header indicating a masked error.
pub fn is_response_success(response: &Response<Vec<u8>>) -> bool {
    let is_hack_error = response.header("x-original-status").is_some();
    response.status().is_success() && !is_hack_error
}

/// Extracts error message from HTTP response.
///
/// Checks for shell hack header first, then falls back to body content.
pub fn extract_error_message(action: &str, response: &mut Response<Vec<u8>>) -> String {
    // Check for original status header from shell hack
    let status = if let Some(original) = response.header("x-original-status") {
        original.as_str().to_string()
    } else {
        response.status().to_string()
    };

    match response.take_body() {
        Some(body) => {
            if body.is_empty() {
                format!("{action} failed: HTTP {status} (Empty body)")
            } else {
                match String::from_utf8(body) {
                    Ok(msg) => format!("Error: {msg}"),
                    Err(e) => format!("{action} failed: HTTP {status} (Invalid UTF-8: {e})"),
                }
            }
        }
        None => format!("{action} failed: HTTP {status} (No body)"),
    }
}

/// Parse JSON from response body.
///
/// Returns error if response is not successful or JSON parsing fails.
pub fn parse_json_response<T: serde::de::DeserializeOwned>(
    action: &str,
    response: &mut Response<Vec<u8>>,
) -> Result<T, String> {
    if !is_response_success(response) {
        return Err(extract_error_message(action, response));
    }

    match response.take_body() {
        Some(body) => {
            serde_json::from_slice(&body).map_err(|e| format!("{action}: JSON parse error: {e}"))
        }
        None => Err(format!("{action}: Empty response body")),
    }
}

/// Check response status only (no body parsing).
///
/// For endpoints that return status-only responses.
pub fn check_response_status(action: &str, response: &mut Response<Vec<u8>>) -> Result<(), String> {
    if is_response_success(response) {
        Ok(())
    } else {
        Err(extract_error_message(action, response))
    }
}

/// Extract string body from response.
///
/// For endpoints that return plain text (e.g., auth tokens).
pub fn extract_string_response(
    action: &str,
    response: &mut Response<Vec<u8>>,
) -> Result<String, String> {
    if !is_response_success(response) {
        return Err(extract_error_message(action, response));
    }

    match response.take_body() {
        Some(bytes) => {
            String::from_utf8(bytes).map_err(|_| format!("{action}: Invalid UTF-8 in response"))
        }
        None => Err(format!("{action}: Empty response body")),
    }
}

/// Process HTTP response result and check status only (no JSON parsing)
pub fn process_status_response(
    action: &str,
    result: crux_http::Result<Response<Vec<u8>>>,
) -> Result<(), String> {
    match result {
        Ok(mut response) => check_response_status(action, &mut response),
        Err(e) => Err(e.to_string()),
    }
}

/// Process HTTP response result and parse JSON
pub fn process_json_response<T: serde::de::DeserializeOwned>(
    action: &str,
    result: crux_http::Result<Response<Vec<u8>>>,
) -> Result<T, String> {
    match result {
        Ok(mut response) => parse_json_response(action, &mut response),
        Err(e) => Err(e.to_string()),
    }
}

/// Handle authentication error - sets error message and returns render command
///
/// This is a common pattern used throughout the codebase when authentication is required
/// but no token is available.
pub fn handle_auth_error<M, E>(model: &mut M, action: &str) -> crux_core::Command<crate::Effect, E>
where
    M: crate::model::ModelErrorHandler,
    E: Send + 'static,
{
    model.set_error(format!("{action} failed: Not authenticated"));
    crux_core::render::render()
}

/// Handle request creation error - sets error message and returns render command
///
/// This is used when building an HTTP request fails (e.g., JSON serialization error).
pub fn handle_request_error<M, E>(
    model: &mut M,
    action: &str,
    error: impl std::fmt::Display,
) -> crux_core::Command<crate::Effect, E>
where
    M: crate::model::ModelErrorHandler,
    E: Send + 'static,
{
    model.set_error(format!("Failed to create {action} request: {error}"));
    crux_core::render::render()
}

// Note: Unit tests for these helpers are not included because crux_http::Response
// has a private constructor. These functions are integration-tested through the
// macros that use them. If crux_http provides a test utility for constructing
// Response objects in the future, we can add unit tests here.
