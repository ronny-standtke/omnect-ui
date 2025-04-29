use crate::socket_client::*;
use crate::{middleware::TOKEN_EXPIRE_HOURS, validate_password};
use actix_files::NamedFile;
use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use actix_session::Session;
use actix_web::{web, HttpResponse, Responder};
use anyhow::{bail, Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use jwt_simple::prelude::*;
use log::{debug, error};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::{
    fs::{self, File},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

macro_rules! data_path {
    ($filename:expr) => {{
        Path::new("/data/").join($filename)
    }};
}

macro_rules! config_path {
    ($filename:expr) => {{
        Path::new("/data/").join("config/").join($filename)
    }};
}

macro_rules! tmp_path {
    ($filename:expr) => {{
        Path::new("/tmp/").join($filename)
    }};
}

#[derive(Deserialize)]
pub struct FactoryResetInput {
    preserve: Vec<String>,
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

#[derive(Serialize)]
pub struct FactoryResetPayload {
    mode: FactoryResetMode,
    preserve: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct LoadUpdatePayload {
    update_file_path: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct RunUpdatePayload {
    validate_iothub_connection: bool,
}

#[derive(MultipartForm)]
pub struct UploadFormSingleFile {
    file: TempFile,
}

#[derive(Clone, Debug, Deserialize_repr, PartialEq, Serialize_repr)]
#[repr(u8)]
pub enum FactoryResetMode {
    Mode1 = 1,
    Mode2 = 2,
    Mode3 = 3,
    Mode4 = 4,
}

#[derive(Clone)]
pub struct Api {
    pub ods_socket_path: String,
    pub update_os_path: String,
    pub centrifugo_client_token_hmac_secret_key: String,
    pub index_html: PathBuf,
}

impl Api {
    pub async fn index(config: web::Data<Api>) -> actix_web::Result<NamedFile> {
        debug!("index() called");

        static ENDPOINT: &str = concat!("/republish/v1/", env!("CARGO_PKG_NAME"));

        if let Err(e) = post_with_empty_body(ENDPOINT, &config.ods_socket_path).await {
            error!("republish failed: {e:#}");
            return Err(actix_web::error::ErrorInternalServerError(
                "republish failed",
            ));
        }

        Ok(NamedFile::open(&config.index_html)?)
    }

    pub async fn factory_reset(
        body: web::Json<FactoryResetInput>,
        config: web::Data<Api>,
    ) -> impl Responder {
        debug!(
            "factory_reset() called with preserved keys {}",
            body.preserve.join(",")
        );

        let payload = FactoryResetPayload {
            mode: FactoryResetMode::Mode1,
            preserve: body.preserve.clone(),
        };

        match post_with_json_body("/factory-reset/v1", payload, &config.ods_socket_path).await {
            Ok(response) => response,
            Err(e) => {
                error!("factory_reset failed: {e:#}");
                HttpResponse::InternalServerError().body(format!("{e}"))
            }
        }
    }

    pub async fn reboot(config: web::Data<Api>) -> impl Responder {
        debug!("reboot() called");

        match post_with_empty_body("/reboot/v1", &config.ods_socket_path).await {
            Ok(response) => response,
            Err(e) => {
                error!("reboot failed: {e:#}");
                HttpResponse::InternalServerError().body(format!("{e}"))
            }
        }
    }

    pub async fn reload_network(config: web::Data<Api>) -> impl Responder {
        debug!("reload_network() called");

        match post_with_empty_body("/reload-network/v1", &config.ods_socket_path).await {
            Ok(response) => response,
            Err(e) => {
                error!("reload-network failed: {e:#}");
                HttpResponse::InternalServerError().body(format!("{e}"))
            }
        }
    }

    pub async fn token(session: Session) -> impl Responder {
        if let Ok(key) = std::env::var("CENTRIFUGO_CLIENT_TOKEN_HMAC_SECRET_KEY") {
            let key = HS256Key::from_bytes(key.as_bytes());
            let claims =
                Claims::create(Duration::from_hours(TOKEN_EXPIRE_HOURS)).with_subject("omnect-ui");

            if let Ok(token) = key.authenticate(claims) {
                match session.insert("token", token.clone()) {
                    Ok(_) => return HttpResponse::Ok().body(token),
                    Err(e) => return HttpResponse::InternalServerError().body(format!("{e}")),
                }
            } else {
                error!("token: cannot create token");
            };
        } else {
            error!("token: missing secret key");
        };

        HttpResponse::InternalServerError().finish()
    }

    pub async fn logout(session: Session) -> impl Responder {
        debug!("logout() called");
        session.purge();
        HttpResponse::Ok()
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

        let _ = Api::clear_data_folder().await;

        if let Err(e) =
            Api::persist_uploaded_file(form.file, &tmp_path!(&filename), &data_path!(&filename))
                .await
        {
            error!("save_file() failed: {e:#}");
            return HttpResponse::InternalServerError().body(format!("{e}"));
        }

        if let Err(e) = Api::set_file_permission(&data_path!(&filename)).await {
            error!("save_file() failed: {e:#}");
            return HttpResponse::InternalServerError().body(format!("{e}"));
        }

        HttpResponse::Ok().finish()
    }

    pub async fn load_update(
        mut body: web::Json<LoadUpdatePayload>,
        config: web::Data<Api>,
    ) -> impl Responder {
        debug!("load_update() called with path {:?}", body.update_file_path);

        body.update_file_path = Path::new(&config.update_os_path).join(&body.update_file_path);

        match post_with_json_body("/fwupdate/load/v1", body, &config.ods_socket_path).await {
            Ok(response) => response,
            Err(e) => {
                error!("load_update failed: {e:#}");
                HttpResponse::InternalServerError().body(format!("{e}"))
            }
        }
    }

    pub async fn run_update(
        body: web::Json<RunUpdatePayload>,
        config: web::Data<Api>,
    ) -> impl Responder {
        debug!(
            "run_update() called with validate_iothub_connection: {}",
            body.validate_iothub_connection
        );

        match post_with_json_body("/fwupdate/run/v1", body, &config.ods_socket_path).await {
            Ok(response) => response,
            Err(e) => {
                error!("run_update failed: {e:#}");
                HttpResponse::InternalServerError().body(format!("{e}"))
            }
        }
    }

    pub async fn set_password(body: web::Json<SetPasswordPayload>) -> impl Responder {
        debug!("set_password() called");

        if !Api::set_password_necessary() {
            return HttpResponse::Found()
                .append_header(("Location", "/login"))
                .finish();
        }

        if let Err(e) = Api::store_or_update_password(&body.password).await {
            error!("set_password() failed: {e:#}");
            return HttpResponse::InternalServerError().body(format!("{:#}", e));
        }

        HttpResponse::Ok().finish()
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

        if let Err(e) = Api::store_or_update_password(&body.password).await {
            error!("update_password() failed: {e:#}");
            return HttpResponse::InternalServerError().body(format!("{:#}", e));
        }

        session.purge();
        HttpResponse::Ok().finish()
    }

    pub async fn require_set_password() -> impl Responder {
        debug!("require_set_password() called");

        if Api::set_password_necessary() {
            return HttpResponse::Created()
                .append_header(("Location", "/set-password"))
                .finish();
        }

        HttpResponse::Ok().finish()
    }

    async fn clear_data_folder() -> Result<()> {
        debug!("clear_data_folder() called");
        for entry in fs::read_dir("/data")? {
            let entry = entry?;
            if entry.path().is_file() {
                fs::remove_file(entry.path())?;
            }
        }

        Ok(())
    }

    async fn persist_uploaded_file(
        tmp_file: TempFile,
        temp_path: &Path,
        data_path: &Path,
    ) -> Result<()> {
        debug!("persist_uploaded_file() called");

        tmp_file
            .file
            .persist(temp_path)
            .context("failed to persist tmp file")?;

        fs::copy(temp_path, data_path).context("failed to copy file to data dir")?;

        Ok(())
    }

    async fn set_file_permission(file_path: &Path) -> Result<()> {
        debug!("set_file_permission() called");

        let metadata = fs::metadata(file_path).context("failed to get file metadata")?;
        let mut perm = metadata.permissions();
        perm.set_mode(0o750);
        fs::set_permissions(file_path, perm).context("failed to set file permission")?;

        Ok(())
    }

    fn set_password_necessary() -> bool {
        debug!("set_password_necessary() called");
        let password_file = config_path!("password");
        !password_file.exists()
    }

    async fn hash_password(password: &str) -> Result<String> {
        debug!("hash_password() called");

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        match argon2.hash_password(password.as_bytes(), &salt) {
            Ok(hash) => Ok(hash.to_string()),
            Err(e) => {
                error!("hash_password() failed: {:#}", e);
                bail!("failed to hash password");
            }
        }
    }

    async fn store_or_update_password(password: &str) -> Result<()> {
        debug!("store_or_update_password() called");

        let password_file = config_path!("password");

        if Api::set_password_necessary() {
            if let Some(parent) = password_file.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    error!("failed to create password directory: {:#}", e);
                    bail!("failed to create password directory: {:#}", e);
                }
            } else {
                error!("failed to get parent directory for password file");
                bail!("failed to get parent directory for password file");
            }
        }

        let Ok(hash) = Api::hash_password(password).await else {
            error!("failed to hash password");
            bail!("failed to hash password");
        };

        let Ok(mut file) = File::create(&password_file) else {
            error!("failed to create password file");
            bail!("failed to create password file");
        };

        if let Err(e) = file.write_all(hash.as_bytes()) {
            error!("failed to write password file: {:#}", e);
            bail!("failed to write password file: {:#}", e);
        }

        Ok(())
    }
}
