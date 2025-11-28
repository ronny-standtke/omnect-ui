/// Macro for model field updates with automatic rendering.
/// Supports both single and multiple field updates.
///
/// # Examples
///
/// Single field update:
/// ```ignore
/// update_field!(model.system_info, Some(info))
/// ```
///
/// Multiple field updates:
/// ```ignore
/// update_field!(
///     model.is_connected, true;
///     model.error_message, None
/// )
/// ```
#[macro_export]
macro_rules! update_field {
    // Multiple field updates (must come first to match the pattern)
    ($($model_field:expr, $value:expr);+ $(;)?) => {{
        $(
            $model_field = $value;
        )+
        crux_core::render::render()
    }};

    // Single field update
    ($model_field:expr, $value:expr) => {{
        $model_field = $value;
        crux_core::render::render()
    }};
}

/// Macro for parsing JSON channel messages with error handling.
/// Reduces repetitive JSON parsing in WebSocket message handlers.
///
/// # Example
///
/// ```ignore
/// parse_channel_data! {
///     channel, data, model,
///     "SystemInfoV1" => system_info: SystemInfo,
///     "NetworkStatusV1" => network_status: NetworkStatus,
///     "OnlineStatusV1" => online_status: OnlineStatus,
/// }
/// ```
#[macro_export]
macro_rules! parse_channel_data {
    ($channel:expr, $data:expr, $model:expr, $($channel_name:literal => $field:ident: $type:ty),+ $(,)?) => {
        match $channel {
            $(
                $channel_name => {
                    if let Ok(parsed) = serde_json::from_str::<$type>($data) {
                        $model.$field = Some(parsed);
                    }
                }
            )+
            _ => {
                // Unknown channel, ignore
            }
        }
    };
}

/// Helper function for standardized HTTP error messages
pub fn http_error(action: &str, status: impl std::fmt::Display) -> String {
    format!("{action} failed: HTTP {status}")
}

/// Macro for unauthenticated POST requests with standard error handling.
/// Used for login, password setup, and other pre-authentication endpoints.
///
/// # Patterns
///
/// Pattern 1: POST with JSON body expecting JSON response
/// ```ignore
/// unauth_post!(model, "/api/token/login", LoginResponse, "Login",
///     body_json: &credentials,
///     expect_json: AuthToken
/// )
/// ```
///
/// Pattern 2: POST with JSON body expecting status only
/// ```ignore
/// unauth_post!(model, "/api/token/set-password", SetPasswordResponse, "Set password",
///     body_json: &request
/// )
/// ```
///
/// Pattern 3: GET expecting JSON response
/// ```ignore
/// unauth_post!(model, "/api/token/requires-password-set", CheckRequiresPasswordSetResponse, "Check password",
///     method: get,
///     expect_json: bool
/// )
/// ```
#[macro_export]
macro_rules! unauth_post {
    // Pattern 1: POST with JSON body expecting JSON response
    ($model:expr, $endpoint:expr, $response_event:ident, $action:expr, body_json: $body:expr, expect_json: $response_type:ty) => {{
        $model.is_loading = true;
        crux_core::Command::all([
            crux_core::render::render(),
            $crate::HttpCmd::post(format!("{}{}", $crate::API_BASE_URL, $endpoint))
                .header("Content-Type", "application/json")
                .body_json($body)
                .expect(&format!("Failed to serialize {} request", $action))
                .expect_json::<$response_type>()
                .build()
                .then_send(|result| match result {
                    Ok(mut response) => match response.take_body() {
                        Some(data) => $crate::Event::$response_event(Ok(data)),
                        None => {
                            $crate::Event::$response_event(Err("Empty response body".to_string()))
                        }
                    },
                    Err(e) => $crate::Event::$response_event(Err(e.to_string())),
                }),
        ])
    }};

    // Pattern 2: POST with JSON body expecting status only
    ($model:expr, $endpoint:expr, $response_event:ident, $action:expr, body_json: $body:expr) => {{
        $model.is_loading = true;
        crux_core::Command::all([
            crux_core::render::render(),
            $crate::HttpCmd::post(format!("{}{}", $crate::API_BASE_URL, $endpoint))
                .header("Content-Type", "application/json")
                .body_json($body)
                .expect(&format!("Failed to serialize {} request", $action))
                .build()
                .then_send(|result| match result {
                    Ok(response) => {
                        if response.status().is_success() {
                            $crate::Event::$response_event(Ok(()))
                        } else {
                            $crate::Event::$response_event(Err($crate::macros::http_error(
                                $action,
                                response.status(),
                            )))
                        }
                    }
                    Err(e) => $crate::Event::$response_event(Err(e.to_string())),
                }),
        ])
    }};

    // Pattern 3: GET expecting JSON response
    ($model:expr, $endpoint:expr, $response_event:ident, $action:expr, method: get, expect_json: $response_type:ty) => {{
        $model.is_loading = true;
        crux_core::Command::all([
            crux_core::render::render(),
            $crate::HttpCmd::get(format!("{}{}", $crate::API_BASE_URL, $endpoint))
                .expect_json::<$response_type>()
                .build()
                .then_send(|result| match result {
                    Ok(mut response) => match response.take_body() {
                        Some(data) => $crate::Event::$response_event(Ok(data)),
                        None => {
                            $crate::Event::$response_event(Err("Empty response body".to_string()))
                        }
                    },
                    Err(e) => $crate::Event::$response_event(Err(e.to_string())),
                }),
        ])
    }};
}

/// Macro for authenticated POST requests with standard error handling.
/// Reduces boilerplate for POST requests that require authentication.
///
/// # Patterns
///
/// Pattern 1: Simple POST without body
/// ```ignore
/// auth_post!(model, "/api/device/reboot", RebootResponse, "Reboot")
/// ```
///
/// Pattern 2: POST with JSON body
/// ```ignore
/// auth_post!(model, "/api/device/factory-reset", FactoryResetResponse, "Factory reset",
///     body_json: &FactoryResetRequest { mode, preserve }
/// )
/// ```
///
/// Pattern 3: POST with string body
/// ```ignore
/// auth_post!(model, "/api/device/network", SetNetworkConfigResponse, "Set network config",
///     body_string: config
/// )
/// ```
#[macro_export]
macro_rules! auth_post {
    // Pattern 1: Simple POST without body
    ($model:expr, $endpoint:expr, $response_event:ident, $action:expr) => {{
        $model.is_loading = true;
        if let Some(token) = &$model.auth_token {
            crux_core::Command::all([
                crux_core::render::render(),
                $crate::HttpCmd::post(format!("{}{}", $crate::API_BASE_URL, $endpoint))
                    .header("Authorization", format!("Bearer {token}"))
                    .build()
                    .then_send(|result| match result {
                        Ok(response) => {
                            if response.status().is_success() {
                                $crate::Event::$response_event(Ok(()))
                            } else {
                                $crate::Event::$response_event(Err($crate::macros::http_error(
                                    $action,
                                    response.status(),
                                )))
                            }
                        }
                        Err(e) => $crate::Event::$response_event(Err(e.to_string())),
                    }),
            ])
        } else {
            crux_core::render::render()
        }
    }};

    // Pattern 2: POST with JSON body
    ($model:expr, $endpoint:expr, $response_event:ident, $action:expr, body_json: $body:expr) => {{
        $model.is_loading = true;
        if let Some(token) = &$model.auth_token {
            crux_core::Command::all([
                crux_core::render::render(),
                $crate::HttpCmd::post(format!("{}{}", $crate::API_BASE_URL, $endpoint))
                    .header("Authorization", format!("Bearer {token}"))
                    .header("Content-Type", "application/json")
                    .body_json($body)
                    .expect(&format!("Failed to serialize {} request", $action))
                    .build()
                    .then_send(|result| match result {
                        Ok(response) => {
                            if response.status().is_success() {
                                $crate::Event::$response_event(Ok(()))
                            } else {
                                $crate::Event::$response_event(Err($crate::macros::http_error(
                                    $action,
                                    response.status(),
                                )))
                            }
                        }
                        Err(e) => $crate::Event::$response_event(Err(e.to_string())),
                    }),
            ])
        } else {
            crux_core::render::render()
        }
    }};

    // Pattern 3: POST with string body
    ($model:expr, $endpoint:expr, $response_event:ident, $action:expr, body_string: $body:expr) => {{
        $model.is_loading = true;
        if let Some(token) = &$model.auth_token {
            crux_core::Command::all([
                crux_core::render::render(),
                $crate::HttpCmd::post(format!("{}{}", $crate::API_BASE_URL, $endpoint))
                    .header("Authorization", format!("Bearer {token}"))
                    .header("Content-Type", "application/json")
                    .body_string($body)
                    .build()
                    .then_send(|result| match result {
                        Ok(response) => {
                            if response.status().is_success() {
                                $crate::Event::$response_event(Ok(()))
                            } else {
                                $crate::Event::$response_event(Err($crate::macros::http_error(
                                    $action,
                                    response.status(),
                                )))
                            }
                        }
                        Err(e) => $crate::Event::$response_event(Err(e.to_string())),
                    }),
            ])
        } else {
            crux_core::render::render()
        }
    }};
}

/// Macro for handling response events with standard loading state and error handling.
///
/// # Patterns
///
/// Pattern 1: Only success message (for `Result<(), String>`)
/// ```ignore
/// handle_response!(model, result, {
///     success_message: "Operation successful",
/// })
/// ```
///
/// Pattern 2: Custom success handling
/// ```ignore
/// handle_response!(model, result, {
///     on_success: |m, value| {
///         m.some_field = value;
///     },
/// })
/// ```
///
/// Pattern 3: Custom success handler + success message
/// ```ignore
/// handle_response!(model, result, {
///     on_success: |m, value| {
///         m.some_field = value;
///     },
///     success_message: "Operation successful",
/// })
/// ```
///
/// Pattern 4: Custom success handler without loading state (for responses that don't set loading)
/// ```ignore
/// handle_response!(model, result, {
///     on_success: |m, info| {
///         m.healthcheck = Some(info);
///     },
///     no_loading: true,
/// })
/// ```
#[macro_export]
macro_rules! handle_response {
    // Pattern 1: Only success message (for Result<(), String>)
    ($model:expr, $result:expr, {
        success_message: $msg:expr $(,)?
    }) => {{
        $model.is_loading = false;
        match $result {
            Ok(()) => {
                $model.success_message = Some($msg.to_string());
            }
            Err(e) => {
                $model.error_message = Some(e);
            }
        }
        crux_core::render::render()
    }};

    // Pattern 2: Only custom success handler
    ($model:expr, $result:expr, {
        on_success: |$success_model:ident, $value:tt| $success_body:block $(,)?
    }) => {{
        $model.is_loading = false;
        match $result {
            Ok($value) => {
                #[allow(clippy::redundant_locals)]
                let $success_model = $model;
                $success_body
            }
            Err(e) => {
                $model.error_message = Some(e);
            }
        }
        crux_core::render::render()
    }};

    // Pattern 3: Custom success handler + success message
    ($model:expr, $result:expr, {
        on_success: |$success_model:ident, $value:tt| $success_body:block,
        success_message: $msg:expr $(,)?
    }) => {{
        $model.is_loading = false;
        match $result {
            Ok($value) => {
                #[allow(clippy::redundant_locals)]
                let $success_model = $model;
                $success_body
                $model.success_message = Some($msg.to_string());
            }
            Err(e) => {
                $model.error_message = Some(e);
            }
        }
        crux_core::render::render()
    }};

    // Pattern 4: Only on_success without loading state (for HealthcheckResponse)
    ($model:expr, $result:expr, {
        on_success: |$success_model:ident, $value:tt| $success_body:block,
        no_loading: true $(,)?
    }) => {{
        match $result {
            Ok($value) => {
                #[allow(clippy::redundant_locals)]
                let $success_model = $model;
                $success_body
            }
            Err(e) => {
                $model.error_message = Some(e);
            }
        }
        crux_core::render::render()
    }};
}
