use crate::{
    auth::{TokenManager, validate_password},
    config::AppConfig,
    keycloak_client::SingleSignOnProvider,
    network::{NetworkConfig, NetworkConfigService},
    omnect_device_service_client::{DeviceServiceClient, FactoryReset, LoadUpdate, RunUpdate},
};
use actix_files::NamedFile;
use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use actix_session::Session;
use actix_web::{HttpResponse, Responder, web};
use anyhow::{Context, Result, anyhow, bail};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use log::{debug, error};
use serde::Deserialize;
use std::{
    fs::{self, File},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::Path,
};

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
    /// Helper to handle service client results with consistent error logging
    fn handle_service_result(result: Result<()>, operation: &str) -> HttpResponse {
        match result {
            Ok(_) => HttpResponse::Ok().finish(),
            Err(e) => {
                error!("{operation} failed: {e:#}");
                HttpResponse::InternalServerError().body(e.to_string())
            }
        }
    }

    pub async fn new(service_client: ServiceClient, single_sign_on: SingleSignOn) -> Result<Self> {
        Ok(Api {
            service_client,
            single_sign_on,
        })
    }

    pub async fn index(api: web::Data<Self>) -> actix_web::Result<NamedFile> {
        debug!("index() called");

        if let Err(e) = api.service_client.republish().await {
            error!("republish failed: {e:#}");
            return Err(actix_web::error::ErrorInternalServerError(
                "republish failed",
            ));
        }

        Ok(NamedFile::open(&AppConfig::get().paths.index_html)?)
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
                error!("healthcheck: {e:#}");
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

        match api.service_client.factory_reset(body.into_inner()).await {
            Ok(_) => {
                session.purge();
                HttpResponse::Ok().finish()
            }
            Err(e) => {
                error!("factory_reset: {e:#}");
                HttpResponse::InternalServerError().body(e.to_string())
            }
        }
    }

    pub async fn reboot(api: web::Data<Self>) -> impl Responder {
        debug!("reboot() called");
        Self::handle_service_result(api.service_client.reboot().await, "reboot")
    }

    pub async fn reload_network(api: web::Data<Self>) -> impl Responder {
        debug!("reload_network() called");
        Self::handle_service_result(api.service_client.reload_network().await, "reload_network")
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

    pub async fn save_file(
        MultipartForm(form): MultipartForm<UploadFormSingleFile>,
    ) -> impl Responder {
        debug!("save_file() called");

        let Some(filename) = form.file.file_name.clone() else {
            return HttpResponse::BadRequest().body("update file is missing");
        };

        if let Err(e) = Self::clear_data_folder() {
            error!("failed to clear data folder: {e:#}");
            // Continue anyway as this is not critical
        }

        if let Err(e) = Self::persist_uploaded_file(
            form.file,
            &AppConfig::get().paths.tmp_dir.join(&filename),
            &AppConfig::get().paths.update_file_internal,
        ) {
            error!("failed to save uploaded file: {e:#}");
            return HttpResponse::InternalServerError().body(e.to_string());
        }

        HttpResponse::Ok().finish()
    }

    pub async fn load_update(api: web::Data<Self>) -> impl Responder {
        debug!("load_update() called with path");

        match api
            .service_client
            .load_update(LoadUpdate {
                update_file_path: AppConfig::get().paths.update_file.clone(),
            })
            .await
        {
            Ok(data) => HttpResponse::Ok().body(data),
            Err(e) => {
                error!("load_update failed: {e:#}");
                HttpResponse::InternalServerError().body(e.to_string())
            }
        }
    }

    pub async fn run_update(body: web::Json<RunUpdate>, api: web::Data<Self>) -> impl Responder {
        debug!("run_update() called with validate_iothub_connection: {body:?}");
        Self::handle_service_result(
            api.service_client.run_update(body.into_inner()).await,
            "run_update",
        )
    }

    pub async fn set_password(
        body: web::Json<SetPasswordPayload>,
        session: Session,
        token_manager: web::Data<TokenManager>,
    ) -> impl Responder {
        debug!("set_password() called");

        if AppConfig::get().paths.password_file.exists() {
            return HttpResponse::Found()
                .append_header(("Location", "/login"))
                .finish();
        }

        if let Err(e) = Self::store_or_update_password(&body.password) {
            error!("set_password() failed: {e:#}");
            return HttpResponse::InternalServerError().body(e.to_string());
        }

        Self::session_token(session, token_manager)
    }

    pub async fn update_password(
        body: web::Json<UpdatePasswordPayload>,
        session: Session,
    ) -> impl Responder {
        debug!("update_password() called");

        if let Err(e) = validate_password(&body.current_password) {
            error!("update_password() failed: {e:#}");
            return HttpResponse::BadRequest().body("current password is not correct");
        }

        if let Err(e) = Self::store_or_update_password(&body.password) {
            error!("update_password() failed: {e:#}");
            return HttpResponse::InternalServerError().body(e.to_string());
        }

        session.purge();
        HttpResponse::Ok().finish()
    }

    pub async fn require_set_password() -> impl Responder {
        debug!("require_set_password() called");

        if !AppConfig::get().paths.password_file.exists() {
            return HttpResponse::Created()
                .append_header(("Location", "/set-password"))
                .finish();
        }

        HttpResponse::Ok().finish()
    }

    pub async fn validate_portal_token(body: String, api: web::Data<Self>) -> impl Responder {
        debug!("validate_portal_token() called");
        if let Err(e) = api.validate_token_and_claims(&body).await {
            error!("validate_portal_token() failed: {e:#}");
            return HttpResponse::Unauthorized().finish();
        }
        HttpResponse::Ok().finish()
    }

    pub async fn set_network_config(
        network_config: web::Json<NetworkConfig>,
        api: web::Data<Self>,
    ) -> impl Responder {
        debug!("set_network_config() called");

        Self::handle_service_result(
            NetworkConfigService::set_network_config(&api.service_client, &network_config).await,
            "set_network_config",
        )
    }

    async fn validate_token_and_claims(&self, token: &str) -> Result<()> {
        let claims = self.single_sign_on.verify_token(token).await?;
        let Some(tenant_list) = &claims.tenant_list else {
            bail!("failed to authorize user: no tenant list in token");
        };
        if !tenant_list.contains(&AppConfig::get().tenant) {
            bail!("failed to authorize user: insufficient permissions for tenant");
        }
        let Some(roles) = &claims.roles else {
            bail!("failed to authorize user: no roles in token");
        };
        if roles.contains(&String::from("FleetAdministrator")) {
            return Ok(());
        }
        if roles.contains(&String::from("FleetOperator")) {
            let Some(fleet_list) = &claims.fleet_list else {
                bail!("failed to authorize user: no fleet list in token");
            };
            let fleet_id = self.service_client.fleet_id().await?;
            if !fleet_list.contains(&fleet_id) {
                bail!("failed to authorize user: insufficient permissions for fleet");
            }
            return Ok(());
        }
        bail!("failed to authorize user: insufficient role permissions")
    }

    fn clear_data_folder() -> Result<()> {
        debug!("clear_data_folder() called");
        for entry in fs::read_dir("/data")? {
            let entry = entry?;
            if entry.path().is_file() {
                fs::remove_file(entry.path())?;
            }
        }

        Ok(())
    }

    fn persist_uploaded_file(tmp_file: TempFile, temp_path: &Path, data_path: &Path) -> Result<()> {
        debug!("persist_uploaded_file() called");

        tmp_file
            .file
            .persist(temp_path)
            .context("failed to persist tmp file")?;

        fs::copy(temp_path, data_path).context("failed to copy file to data dir")?;

        let metadata = fs::metadata(data_path).context("failed to get file metadata")?;
        let mut perm = metadata.permissions();
        perm.set_mode(0o750);
        fs::set_permissions(data_path, perm).context("failed to set file permission")
    }

    fn hash_password(password: &str) -> Result<String> {
        debug!("hash_password() called");

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        match argon2.hash_password(password.as_bytes(), &salt) {
            Ok(hash) => Ok(hash.to_string()),
            Err(e) => Err(anyhow!(e).context("failed to hash password")),
        }
    }

    fn store_or_update_password(password: &str) -> Result<()> {
        debug!("store_or_update_password() called");
        let hash = Self::hash_password(password)?;
        let mut file = File::create(&AppConfig::get().paths.password_file)
            .context("failed to create password file")?;

        file.write_all(hash.as_bytes())
            .context("failed to write password file")
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
