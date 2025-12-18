use base64::prelude::*;
use crux_core::Command;

use crate::auth_post;
use crate::auth_post_basic;
use crate::events::{AuthEvent, Event};
use crate::handle_response;
use crate::model::Model;
use crate::types::{AuthToken, SetPasswordRequest, UpdatePasswordRequest};
use crate::unauth_post;
use crate::Effect;

/// Handle authentication-related events
pub fn handle(event: AuthEvent, model: &mut Model) -> Command<Effect, Event> {
    match event {
        AuthEvent::Login { password } => {
            let encoded = BASE64_STANDARD.encode(format!(":{password}"));
            auth_post_basic!(Auth, AuthEvent, model, "/token/login", LoginResponse, "Login",
                credentials: encoded,
                map: |token| AuthToken { token })
        }

        AuthEvent::LoginResponse(result) => handle_response!(model, result, {
            on_success: |model, auth| {
                model.auth_token = Some(auth.token);
                model.is_authenticated = true;
            },
        }),

        AuthEvent::Logout => {
            auth_post!(Auth, AuthEvent, model, "/logout", LogoutResponse, "Logout")
        }

        AuthEvent::LogoutResponse(result) => handle_response!(model, result, {
            on_success: |model, _| {
                model.invalidate_session();
            },
        }),

        AuthEvent::SetPassword { password } => {
            let request = SetPasswordRequest { password };
            unauth_post!(Auth, AuthEvent, model, "/set-password", SetPasswordResponse, "Set password",
                body_json: &request
            )
        }

        AuthEvent::SetPasswordResponse(result) => handle_response!(model, result, {
            on_success: |model, _| {
                model.requires_password_set = false;
            },
            success_message: "Password set successfully",
        }),

        AuthEvent::UpdatePassword {
            current_password,
            password,
        } => {
            let request = UpdatePasswordRequest {
                current_password,
                password,
            };
            auth_post!(Auth, AuthEvent, model, "/update-password", UpdatePasswordResponse, "Update password",
                body_json: &request
            )
        }

        AuthEvent::UpdatePasswordResponse(result) => handle_response!(model, result, {
            on_success: |model, _| {},
            success_message: "Password updated successfully",
        }),

        AuthEvent::CheckRequiresPasswordSet => {
            unauth_post!(Auth, AuthEvent, model, "/require-set-password", CheckRequiresPasswordSetResponse, "Check password",
                method: get,
                expect_json: bool
            )
        }

        AuthEvent::CheckRequiresPasswordSetResponse(result) => handle_response!(model, result, {
            on_success: |model, requires| {
                model.requires_password_set = requires;
            },
        }),
    }
}
