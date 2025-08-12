use crate::{
    common::{centrifugo_config, config_path, validate_password},
    keycloak_client::SingleSignOnProvider,
    middleware::TOKEN_EXPIRE_HOURS,
    omnect_device_service_client::*,
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
use jwt_simple::prelude::*;
use log::{debug, error};
use serde::Deserialize;
use std::{
    fs::{self, File},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Arc,
};

macro_rules! data_path {
    ($filename:expr) => {
        Path::new("/data/").join($filename)
    };
}

macro_rules! host_data_path {
    ($filename:expr) => {
        Path::new(&format!("/var/lib/{}/", env!("CARGO_PKG_NAME"))).join($filename)
    };
}

macro_rules! tmp_path {
    ($filename:expr) => {
        Path::new("/tmp/").join($filename)
    };
}

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
    pub service_client: Arc<ServiceClient>,
    pub single_sign_on: Arc<SingleSignOn>,
    pub index_html: PathBuf,
    pub tenant: String,
}

impl<ServiceClient, SingleSignOn> Api<ServiceClient, SingleSignOn>
where
    ServiceClient: DeviceServiceClient,
    SingleSignOn: SingleSignOnProvider,
{
    const UPDATE_FILE_NAME: &str = "update.tar";

    pub async fn new(service_client: ServiceClient, single_sign_on: SingleSignOn) -> Result<Self> {
        let index_html =
            std::fs::canonicalize("static/index.html").context("static/index.html not found")?;
        let tenant = std::env::var("TENANT").unwrap_or("cp".to_string());
        Ok(Api {
            service_client: Arc::new(service_client),
            single_sign_on: Arc::new(single_sign_on),
            index_html,
            tenant,
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

        Ok(NamedFile::open(&api.index_html)?)
    }

    pub async fn config() -> actix_web::Result<NamedFile> {
        Ok(NamedFile::open(config_path!("app_config.js"))?)
    }

    pub async fn healthcheck(api: web::Data<Self>) -> impl Responder {
        debug!("healthcheck() called");

        match api.service_client.version_info().await {
            Ok(info) if info.mismatch => HttpResponse::ServiceUnavailable().json(&info),
            Ok(info) => HttpResponse::Ok().json(&info),
            Err(e) => {
                error!("healthcheck: {e:#}");
                HttpResponse::InternalServerError().body(format!("{e}"))
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
                HttpResponse::InternalServerError().body(format!("{e}"))
            }
        }
    }

    pub async fn reboot(api: web::Data<Self>) -> impl Responder {
        debug!("reboot() called");

        match api.service_client.reboot().await {
            Ok(_) => HttpResponse::Ok().finish(),
            Err(e) => {
                error!("reboot failed: {e:#}");
                HttpResponse::InternalServerError().body(format!("{e}"))
            }
        }
    }

    pub async fn reload_network(api: web::Data<Self>) -> impl Responder {
        debug!("reload_network() called");

        match api.service_client.reload_network().await {
            Ok(_) => HttpResponse::Ok().finish(),
            Err(e) => {
                error!("reload_network failed: {e:#}");
                HttpResponse::InternalServerError().body(format!("{e}"))
            }
        }
    }

    pub async fn token(session: Session) -> impl Responder {
        debug!("token() called");

        Self::session_token(session)
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

        let _ = Self::clear_data_folder();

        if let Err(e) = Self::persist_uploaded_file(
            form.file,
            &tmp_path!(&filename),
            &data_path!(&Self::UPDATE_FILE_NAME),
        ) {
            error!("save_file() failed: {e:#}");
            return HttpResponse::InternalServerError().body(format!("{e}"));
        }

        HttpResponse::Ok().finish()
    }

    pub async fn load_update(api: web::Data<Self>) -> impl Responder {
        debug!("load_update() called with path");

        match api
            .service_client
            .load_update(LoadUpdate {
                update_file_path: host_data_path!(&Self::UPDATE_FILE_NAME)
                    .display()
                    .to_string(),
            })
            .await
        {
            Ok(data) => HttpResponse::Ok().body(data),
            Err(e) => {
                error!("load_update failed: {e:#}");
                HttpResponse::InternalServerError().body(format!("{e}"))
            }
        }
    }

    pub async fn run_update(body: web::Json<RunUpdate>, api: web::Data<Self>) -> impl Responder {
        debug!("run_update() called with validate_iothub_connection: {body:?}");

        match api.service_client.run_update(body.into_inner()).await {
            Ok(_) => HttpResponse::Ok().finish(),
            Err(e) => {
                error!("run_update failed: {e:#}");
                HttpResponse::InternalServerError().body(format!("{e}"))
            }
        }
    }

    pub async fn set_password(
        body: web::Json<SetPasswordPayload>,
        session: Session,
    ) -> impl Responder {
        debug!("set_password() called");

        if config_path!("password").exists() {
            return HttpResponse::Found()
                .append_header(("Location", "/login"))
                .finish();
        }

        if let Err(e) = Self::store_or_update_password(&body.password) {
            error!("set_password() failed: {e:#}");
            return HttpResponse::InternalServerError().body(format!("{e:#}"));
        }

        Self::session_token(session)
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
            return HttpResponse::InternalServerError().body(format!("{e:#}"));
        }

        session.purge();
        HttpResponse::Ok().finish()
    }

    pub async fn require_set_password() -> impl Responder {
        debug!("require_set_password() called");

        if !config_path!("password").exists() {
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

    async fn validate_token_and_claims(&self, token: &str) -> Result<()> {
        let claims = self.single_sign_on.verify_token(token).await?;
        let Some(tenant_list) = &claims.tenant_list else {
            bail!("user has no tenant list");
        };
        if !tenant_list.contains(&self.tenant) {
            bail!("user has no permission to set password");
        }
        let Some(roles) = &claims.roles else {
            bail!("user has no roles");
        };
        if roles.contains(&String::from("FleetAdministrator")) {
            return Ok(());
        }
        if roles.contains(&String::from("FleetOperator")) {
            let Some(fleet_list) = &claims.fleet_list else {
                bail!("user has no permission on this fleet");
            };
            let fleet_id = self.service_client.fleet_id().await?;
            if !fleet_list.contains(&fleet_id) {
                bail!("user has no permission on this fleet");
            }
            return Ok(());
        }
        bail!("user has no permission to set password")
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

        let password_file = config_path!("password");
        let hash = Self::hash_password(password)?;
        let mut file = File::create(&password_file).context("failed to create password file")?;

        file.write_all(hash.as_bytes())
            .context("failed to write password file")
    }

    fn session_token(session: Session) -> HttpResponse {
        let key = HS256Key::from_bytes(centrifugo_config().client_token.as_bytes());
        let claims =
            Claims::create(Duration::from_hours(TOKEN_EXPIRE_HOURS)).with_subject("omnect-ui");

        let Ok(token) = key.authenticate(claims) else {
            error!("failed to create token");
            return HttpResponse::InternalServerError().body("failed to create token");
        };

        if session.insert("token", &token).is_err() {
            error!("failed to insert token into session");
            return HttpResponse::InternalServerError().body("failed to insert token into session");
        }

        HttpResponse::Ok().body(token)
    }
}
