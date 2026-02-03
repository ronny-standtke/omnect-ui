//! HTTP helper functions for Crux Core
//!
//! This module extracts common HTTP response handling logic from macros
//! into debuggable, testable functions.

use crux_http::Response;

/// Base URL for omnect-device API endpoints.
///
/// NOTE: This is a dummy prefix required because `crux_http` (v0.16.0-rc2) requires
/// absolute URLs and rejects relative paths (`RelativeUrlWithoutBase` error).
/// The UI shell (`http.ts`) strips this prefix before sending requests via `fetch()`,
/// making them relative to avoid HTTPS certificate CN/SAN validation issues.
/// Using https:// to prevent any potential mixed content warnings on HTTPS pages.
pub const BASE_URL: &str = "https://relative";

/// Constructs the full address from a given endpoint.
///
/// # Arguments
/// * `endpoint` - The API endpoint path (e.g., "/api/device/reboot")
///
/// # Returns
/// A string containing the full URL with dummy prefix
///
/// # Example
/// ```
/// use omnect_ui_core::http_helpers::build_url;
/// let url = build_url("/api/device/reboot");
/// assert_eq!(url, "https://relative/api/device/reboot");
/// ```
pub fn build_url(endpoint: &str) -> String {
    format!("{BASE_URL}{endpoint}")
}

/// Validates HTTP response.
///
/// Returns `true` if the response status is 2xx.
pub fn is_response_success(response: &Response<Vec<u8>>) -> bool {
    response.status().is_success()
}

/// Extracts error message from HTTP response.
pub fn extract_error_message(action: &str, response: &mut Response<Vec<u8>>) -> String {
    let status = response.status().to_string();

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
