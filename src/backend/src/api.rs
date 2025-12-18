use crate::{
    config::AppConfig,
    http_client::handle_service_result,
    keycloak_client::SingleSignOnProvider,
    omnect_device_service_client::{DeviceServiceClient, FactoryReset, RunUpdate},
    services::{
        auth::{AuthorizationService, PasswordService, TokenManager},
        firmware::FirmwareService,
        network::{NetworkConfigService, SetNetworkConfigRequest},
    },
};
use actix_files::NamedFile;
use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use actix_session::Session;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use log::{debug, error};
use serde::Deserialize;
use std::collections::HashMap;

pub type StaticResources = HashMap<&'static str, static_files::Resource>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPasswordPayload {
    password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePasswordPayload {
    current_password: String,
    password: String,
}

#[derive(MultipartForm)]
pub struct UploadFormSingleFile {
    file: TempFile,
}

#[derive(Clone)]
pub struct Api<ServiceClient, SingleSignOn>
where
    ServiceClient: DeviceServiceClient,
    SingleSignOn: SingleSignOnProvider,
{
    pub service_client: ServiceClient,
    pub single_sign_on: SingleSignOn,
}

impl<ServiceClient, SingleSignOn> Api<ServiceClient, SingleSignOn>
where
    ServiceClient: DeviceServiceClient,
    SingleSignOn: SingleSignOnProvider,
{
    pub async fn new(service_client: ServiceClient, single_sign_on: SingleSignOn) -> Result<Self> {
        Ok(Api {
            service_client,
            single_sign_on,
        })
    }

    pub async fn index(
        api: web::Data<Self>,
        static_resources: web::Data<StaticResources>,
    ) -> actix_web::Result<HttpResponse> {
        debug!("index() called");

        api.service_client.republish().await.map_err(|e| {
            error!("republish failed: {e:#}");
            actix_web::error::ErrorInternalServerError("republish failed")
        })?;

        let Some(index_html) = static_resources.get("index.html") else {
            return Err(actix_web::error::ErrorNotFound(
                "index.html not found in embedded resources",
            ));
        };

        Ok(HttpResponse::Ok()
            .content_type(index_html.mime_type)
            .body(index_html.data.to_vec()))
    }

    pub async fn config() -> actix_web::Result<NamedFile> {
        Ok(NamedFile::open(&AppConfig::get().paths.app_config_path)?)
    }

    pub async fn healthcheck(api: web::Data<Self>) -> impl Responder {
        debug!("healthcheck() called");

        match api.service_client.healthcheck_info().await {
            Ok(info) if info.version_info.mismatch => {
                HttpResponse::ServiceUnavailable().json(&info)
            }
            Ok(info) => HttpResponse::Ok().json(&info),
            Err(e) => {
                error!("healthcheck failed: {e:#}");
                HttpResponse::InternalServerError().body(e.to_string())
            }
        }
    }

    pub async fn factory_reset(
        body: web::Json<FactoryReset>,
        api: web::Data<Self>,
        session: Session,
    ) -> impl Responder {
        debug!("factory_reset() called: {body:?}");

        let result = api.service_client.factory_reset(body.into_inner()).await;

        if result.is_ok() {
            session.purge();
        }

        handle_service_result(result, "factory_reset")
    }

    pub async fn reboot(api: web::Data<Self>) -> impl Responder {
        debug!("reboot() called");
        handle_service_result(api.service_client.reboot().await, "reboot")
    }

    pub async fn token(session: Session, token_manager: web::Data<TokenManager>) -> impl Responder {
        debug!("token() called");

        NetworkConfigService::cancel_rollback();
        Self::session_token(session, token_manager)
    }

    pub async fn logout(session: Session) -> impl Responder {
        debug!("logout() called");
        session.purge();
        HttpResponse::Ok().finish()
    }

    pub async fn version() -> impl Responder {
        HttpResponse::Ok().body(env!("CARGO_PKG_VERSION"))
    }

    pub async fn upload_firmware_file(
        MultipartForm(form): MultipartForm<UploadFormSingleFile>,
    ) -> impl Responder {
        debug!("upload_firmware_file() called");

        handle_service_result(
            FirmwareService::handle_uploaded_firmware(form.file),
            "upload_firmware_file",
        )
    }

    pub async fn load_update(api: web::Data<Self>) -> impl Responder {
        debug!("load_update() called");

        handle_service_result(
            FirmwareService::load_update(&api.service_client).await,
            "load_update",
        )
    }

    pub async fn run_update(body: web::Json<RunUpdate>, api: web::Data<Self>) -> impl Responder {
        debug!("run_update() called with validate_iothub_connection: {body:?}");
        handle_service_result(
            FirmwareService::run_update(&api.service_client, body.into_inner()).await,
            "run_update",
        )
    }

    pub async fn set_password(
        body: web::Json<SetPasswordPayload>,
        session: Session,
        token_manager: web::Data<TokenManager>,
    ) -> impl Responder {
        debug!("set_password() called");

        if PasswordService::password_exists() {
            return HttpResponse::Found()
                .append_header(("Location", "/login"))
                .finish();
        }

        if let Err(e) = PasswordService::store_or_update_password(&body.password) {
            error!("set_password failed: {e:#}");
            return HttpResponse::InternalServerError().body(e.to_string());
        }

        Self::session_token(session, token_manager)
    }

    pub async fn update_password(
        body: web::Json<UpdatePasswordPayload>,
        session: Session,
    ) -> impl Responder {
        debug!("update_password() called");

        if let Err(e) = PasswordService::validate_password(&body.current_password) {
            error!("validate_password failed: {e:#}");
            return HttpResponse::BadRequest().body("current password is not correct");
        }

        let result = PasswordService::store_or_update_password(&body.password);

        if result.is_ok() {
            session.purge();
        }

        handle_service_result(result, "update_password")
    }

    pub async fn require_set_password() -> impl Responder {
        debug!("require_set_password() called");

        let password_exists = PasswordService::password_exists();
        HttpResponse::Ok().json(!password_exists)
    }

    pub async fn validate_portal_token(body: String, api: web::Data<Self>) -> impl Responder {
        debug!("validate_portal_token() called");

        if let Err(e) = AuthorizationService::validate_token_and_claims(
            &api.single_sign_on,
            &api.service_client,
            &body,
        )
        .await
        {
            error!("validate_portal_token failed: {e:#}");
            return HttpResponse::Unauthorized().finish();
        }

        HttpResponse::Ok().finish()
    }

    pub async fn set_network_config(
        network_config: web::Json<SetNetworkConfigRequest>,
        api: web::Data<Self>,
    ) -> impl Responder {
        debug!("set_network_config() called");

        handle_service_result(
            NetworkConfigService::set_network_config(&api.service_client, &network_config).await,
            "set_network_config",
        )
    }

    pub async fn ack_rollback() -> impl Responder {
        debug!("ack_rollback() called");
        NetworkConfigService::clear_rollback_occurred();
        HttpResponse::Ok().finish()
    }

    fn session_token(session: Session, token_manager: web::Data<TokenManager>) -> HttpResponse {
        let token = match token_manager.create_token() {
            Ok(token) => token,
            Err(e) => {
                error!("failed to create token: {e:#}");
                return HttpResponse::InternalServerError().body("failed to create token");
            }
        };

        if session.insert("token", &token).is_err() {
            error!("failed to insert token into session");
            return HttpResponse::InternalServerError().body("failed to insert token into session");
        }

        HttpResponse::Ok().body(token)
    }
}
