use crate::{
    certificate,
    common::{centrifugo_config, config_path, validate_password},
    keycloak_client,
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
use ini::Ini;
use jwt_simple::prelude::*;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration as StdDuration, SystemTime, UNIX_EPOCH},
};
use tokio::{task, time::sleep};

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

macro_rules! network_path {
    ($filename:expr) => {
        Path::new("/network/").join($filename)
    };
}

macro_rules! rollback_file_path {
    () => {
        Path::new("/tmp/network_rollback.json")
    };
}

#[derive(Debug, Deserialize, Serialize)]
struct TokenClaims {
    roles: Option<Vec<String>>,
    tenant_list: Option<Vec<String>>,
    fleet_list: Option<Vec<String>>,
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

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSetting {
    is_server_addr: bool,
    ip_changed: bool,
    name: String,
    dhcp: bool,
    previous_ip: String,
    ip: Option<String>,
    netmask: Option<u8>,
    gateway: Option<Vec<String>>,
    dns: Option<Vec<String>>,
}

#[derive(MultipartForm)]
pub struct UploadFormSingleFile {
    file: TempFile,
}

#[derive(Clone)]
pub struct Api {
    pub ods_client: Arc<OmnectDeviceServiceClient>,
    pub index_html: PathBuf,
    pub tenant: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct PendingNetworkRollback {
    network_setting: NetworkSetting,
    rollback_time: u64,
}

impl Api {
    const UPDATE_FILE_NAME: &str = "update.tar";
    pub async fn new() -> Result<Self> {
        let index_html =
            std::fs::canonicalize("static/index.html").context("static/index.html not found")?;
        let tenant = std::env::var("TENANT").unwrap_or("cp".to_string());
        let ods_client = Arc::new(OmnectDeviceServiceClient::new(true).await?);

        Ok(Api {
            ods_client,
            index_html,
            tenant,
        })
    }

    pub async fn index(api: web::Data<Api>) -> actix_web::Result<NamedFile> {
        debug!("index() called");

        if let Err(e) = api.ods_client.republish().await {
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

    pub async fn healthcheck(api: web::Data<Api>) -> impl Responder {
        debug!("healthcheck() called");

        match api.ods_client.version_info().await {
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
        api: web::Data<Api>,
        session: Session,
    ) -> impl Responder {
        debug!("factory_reset() called: {body:?}");

        match api.ods_client.factory_reset(body.into_inner()).await {
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

    pub async fn reboot(api: web::Data<Api>) -> impl Responder {
        debug!("reboot() called");

        match api.ods_client.reboot().await {
            Ok(_) => HttpResponse::Ok().finish(),
            Err(e) => {
                error!("reboot failed: {e:#}");
                HttpResponse::InternalServerError().body(format!("{e}"))
            }
        }
    }

    pub async fn reload_network(api: web::Data<Api>) -> impl Responder {
        debug!("reload_network() called");

        match api.ods_client.reload_network().await {
            Ok(_) => HttpResponse::Ok().finish(),
            Err(e) => {
                error!("reload_network failed: {e:#}");
                HttpResponse::InternalServerError().body(format!("{e}"))
            }
        }
    }

    pub async fn token(session: Session, api: web::Data<Api>) -> impl Responder {
        debug!("token() called");

        crate::cancel_rollback_timer().await;
        api.cancel_pending_rollback().await;
        Api::session_token(session)
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

        let _ = Api::clear_data_folder();

        if let Err(e) = Api::persist_uploaded_file(
            form.file,
            &tmp_path!(&filename),
            &data_path!(&Api::UPDATE_FILE_NAME),
        ) {
            error!("save_file() failed: {e:#}");
            return HttpResponse::InternalServerError().body(format!("{e}"));
        }

        HttpResponse::Ok().finish()
    }

    pub async fn load_update(api: web::Data<Api>) -> impl Responder {
        debug!("load_update() called with path");

        match api
            .ods_client
            .load_update(LoadUpdate {
                update_file_path: host_data_path!(&Api::UPDATE_FILE_NAME)
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

    pub async fn run_update(body: web::Json<RunUpdate>, api: web::Data<Api>) -> impl Responder {
        debug!("run_update() called with validate_iothub_connection: {body:?}");

        match api.ods_client.run_update(body.into_inner()).await {
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

        if let Err(e) = Api::store_or_update_password(&body.password) {
            error!("set_password() failed: {e:#}");
            return HttpResponse::InternalServerError().body(format!("{e:#}"));
        }

        Api::session_token(session)
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

        if let Err(e) = Api::store_or_update_password(&body.password) {
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

    pub async fn validate_portal_token(body: String, api: web::Data<Api>) -> impl Responder {
        debug!("validate_portal_token() called");

        if let Err(e) = api.validate_token_and_claims(&body).await {
            error!("validate_portal_token() failed: {e:#}");
            return HttpResponse::Unauthorized().finish();
        }
        HttpResponse::Ok().finish()
    }

    pub async fn set_network(
        body: web::Json<NetworkSetting>,
        api: web::Data<Api>,
    ) -> impl Responder {
        debug!("set_network() called");

        if body.name.is_empty() {
            return HttpResponse::BadRequest().body("Network name cannot be empty");
        }

        if let Some(ip) = &body.ip {
            if !ip.is_empty() && ip.parse::<std::net::IpAddr>().is_err() {
                return HttpResponse::BadRequest().body("Invalid IP address format");
            }
        }

        if let Some(netmask) = &body.netmask {
            if *netmask > 32 {
                return HttpResponse::BadRequest().body("Netmask must be between 0 and 32");
            }
        }

        if let Some(gateway) = &body.gateway {
            for gw in gateway {
                if !gw.is_empty() && gw.parse::<std::net::IpAddr>().is_err() {
                    return HttpResponse::BadRequest().body("Invalid gateway format");
                }
            }
        }

        if let Some(dns) = &body.dns {
            for dns_entry in dns {
                if !dns_entry.is_empty() && dns_entry.parse::<std::net::IpAddr>().is_err() {
                    return HttpResponse::BadRequest().body("Invalid DNS format");
                }
            }
        }

        if let Err(e) = api.configure_network_interface(body.clone()).await {
            let _ = api.restore_network_setting(body.into_inner());
            error!("set_network() failed: {e:#}");
            return HttpResponse::InternalServerError().body(format!("{e:#}"));
        }

        HttpResponse::Ok().finish()
    }

    fn restore_network_setting(&self, network: NetworkSetting) -> Result<()> {
        let config_file = network_path!(format!("10-{}.network", network.name));
        let backup_file = network_path!(format!("10-{}.network.old", network.name));

        if fs::exists(&config_file)? {
            fs::remove_file(&config_file)?;
        }

        if fs::exists(&backup_file)? {
            fs::rename(backup_file, config_file)?;
        }

        Ok(())
    }

    async fn restore_network_setting_and_reset_certificate(
        &self,
        network: NetworkSetting,
    ) -> Result<()> {
        let _ = self.restore_network_setting(network.clone());
        let _ = self.ods_client.reload_network().await;
        let _ = certificate::create_module_certificate(Some(network.previous_ip)).await;
        crate::trigger_server_restart();

        Ok(())
    }

    async fn configure_network_interface(&self, network: NetworkSetting) -> Result<()> {
        let config_file = network_path!(format!("10-{}.network", &network.name));
        let backup_file = network_path!(format!("10-{}.network.old", &network.name));

        if Path::new(&backup_file).exists() {
            fs::remove_file(&backup_file).context("Unable to remove backup file")?;
        }

        if Path::new(&config_file).exists() {
            fs::rename(config_file, backup_file).context("Failed to back up file")?;
        } else {
            let status = self.ods_client.status().await?;
            let current_network = status
                .network_status
                .network_interfaces
                .iter()
                .find(|iface| iface.name == network.name)
                .context("Failed to find current network interface")?;

            if Path::new(&current_network.file).exists() {
                fs::rename(&current_network.file, backup_file)
                    .context("Failed to back up current network file")?;
            }
        }

        if network.dhcp {
            self.store_dhcp_network_setting(network.clone())?;
        } else {
            self.store_static_network_setting(network.clone())?;
        }

        let ods_client = Arc::clone(&self.ods_client);

        task::spawn(async move {
            let _ = ods_client.reload_network().await;
        });

        if network.is_server_addr && network.ip_changed {
            self.handle_server_restart_with_new_certificate(network)
                .await?;
        }

        Ok(())
    }

    async fn handle_server_restart_with_new_certificate(
        &self,
        network: NetworkSetting,
    ) -> Result<()> {
        let rollback_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 90; // 90 seconds from now

        let pending_rollback = PendingNetworkRollback {
            network_setting: network.clone(),
            rollback_time,
        };

        if let Err(e) = self.save_pending_rollback(&pending_rollback) {
            error!("Failed to save pending rollback: {e:#}");
        }

        task::spawn(async move {
            if let Err(e) = certificate::create_module_certificate(Some(network.ip.unwrap())).await
            {
                error!("Failed to create certificate with new IP: {e:#}");
                return;
            }

            info!("Certificate updated with new IP. Restarting server...");

            crate::trigger_server_restart();
        });

        Ok(())
    }

    fn save_pending_rollback(&self, rollback: &PendingNetworkRollback) -> Result<()> {
        let rollback_json = serde_json::to_string_pretty(rollback)?;
        fs::write(rollback_file_path!(), rollback_json).context("Failed to write rollback file")?;
        Ok(())
    }

    fn load_pending_rollback(&self) -> Option<PendingNetworkRollback> {
        if let Ok(contents) = fs::read_to_string(rollback_file_path!()) {
            serde_json::from_str(&contents).ok()
        } else {
            None
        }
    }

    fn clear_pending_rollback(&self) {
        let _ = fs::remove_file(rollback_file_path!());
    }

    pub async fn check_and_execute_pending_rollback(&self) -> Result<()> {
        if let Some(pending) = self.load_pending_rollback() {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if now >= pending.rollback_time {
                info!("Executing pending network rollback");

                if let Err(e) = self
                    .restore_network_setting_and_reset_certificate(pending.network_setting)
                    .await
                {
                    error!("Failed to execute pending rollback: {e:#}");
                } else {
                    info!("Pending network rollback executed successfully");
                }

                self.clear_pending_rollback();
            } else {
                let remaining_time = pending.rollback_time - now;
                let network_clone = pending.network_setting.clone();
                let self_clone = self.clone();

                task::spawn(async move {
                    sleep(StdDuration::from_secs(remaining_time)).await;

                    if self_clone.load_pending_rollback().is_some() {
                        if let Err(e) = self_clone
                            .restore_network_setting_and_reset_certificate(network_clone)
                            .await
                        {
                            error!("Failed to execute scheduled rollback: {e:#}");
                        } else {
                            info!("Scheduled network rollback executed successfully");
                        }
                        self_clone.clear_pending_rollback();
                    }
                });
            }
        }
        Ok(())
    }

    pub async fn cancel_pending_rollback(&self) {
        if self.load_pending_rollback().is_some() {
            self.clear_pending_rollback();
            info!("Pending network rollback cancelled");
        }
    }

    fn store_dhcp_network_setting(&self, network: NetworkSetting) -> Result<()> {
        let mut ini = Ini::new();

        ini.with_section(Some("Match".to_owned()))
            .set("Name", &network.name);

        ini.with_section(Some("Network").to_owned())
            .set("DHCP", "yes");

        ini.write_to_file(network_path!(format!("10-{}.network", &network.name)))
            .context("Failed to save new config file")?;

        Ok(())
    }

    fn store_static_network_setting(&self, network: NetworkSetting) -> Result<()> {
        let mut ini = Ini::new();

        ini.with_section(Some("Match".to_owned()))
            .set("Name", &network.name);

        let mut network_section = ini.with_section(Some("Network").to_owned());

        network_section.set(
            "Address",
            format!(
                "{}/{}",
                network.ip.unwrap().as_str(),
                network.netmask.unwrap()
            ),
        );

        if let Some(gateways) = network.gateway {
            for gateway in gateways {
                network_section.add("Gateway", gateway);
            }
        }

        if let Some(dnss) = network.dns {
            for dns in dnss {
                network_section.add("DNS", dns);
            }
        }

        ini.write_to_file(network_path!(format!("10-{}.network", network.name)))?;

        Ok(())
    }

    async fn validate_token_and_claims(&self, token: &str) -> Result<()> {
        let pub_key = keycloak_client::realm_public_key()
            .await
            .context("failed to get public key")?;

        let claims = pub_key
            .verify_token::<TokenClaims>(token, None)
            .context("failed to verify token")?;

        let Some(tenant_list) = &claims.custom.tenant_list else {
            bail!("user has no tenant list");
        };

        if !tenant_list.contains(&self.tenant) {
            bail!("user has no permission to set password");
        }

        let Some(roles) = &claims.custom.roles else {
            bail!("user has no roles");
        };

        if roles.contains(&String::from("FleetAdministrator")) {
            return Ok(());
        }

        if roles.contains(&String::from("FleetOperator")) {
            let Some(fleet_list) = &claims.custom.fleet_list else {
                bail!("user has no permission on this fleet");
            };

            let fleet_id = self
                .ods_client
                .fleet_id()
                .await
                .context("failed to get fleet id")?;

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
        let hash = Api::hash_password(password)?;
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
