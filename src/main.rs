mod api;
mod middleware;
mod socket_client;

use crate::api::Api;
use actix_files::Files;
use actix_multipart::form::MultipartFormConfig;
use actix_session::{
    config::{BrowserSession, CookieContentSecurity},
    storage::CookieSessionStore,
    SessionMiddleware,
};
use actix_web::{
    body::MessageBody,
    cookie::{Key, SameSite},
    web::{self, Data},
    App, HttpResponse, HttpServer, Responder,
};
use anyhow::{bail, Result};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use env_logger::{Builder, Env, Target};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::{fs, path::Path};
use tokio::process::Command;
use tokio::signal::unix::{signal, SignalKind};
use uuid::Uuid;

const UPLOAD_LIMIT_BYTES: usize = 250 * 1024 * 1024;
const MEMORY_LIMIT_BYTES: usize = 10 * 1024 * 1024;

macro_rules! config_path {
    ($filename:expr) => {{
        Path::new("/data/").join("config/").join($filename)
    }};
}

macro_rules! update_os_path {
    () => {{
        static DATA_DIR_PATH_DEFAULT: &'static str = "/var/lib/omnect-ui";
        std::env::var("DATA_DIR_PATH").unwrap_or(DATA_DIR_PATH_DEFAULT.to_string())
    }};
}

macro_rules! centrifugo_http_server_port {
    () => {{
        static CENTRIFUGO_HTTP_SERVER_PORT_DEFAULT: &'static str = "8000";
        std::env::var("CENTRIFUGO_HTTP_SERVER_PORT")
            .unwrap_or(CENTRIFUGO_HTTP_SERVER_PORT_DEFAULT.to_string())
    }};
}

#[derive(Serialize)]
struct CreateCertPayload {
    #[serde(rename = "commonName")]
    common_name: String,
}

#[derive(Debug, Deserialize)]
struct PrivateKey {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    type_name: String,
    bytes: String,
}

#[derive(Debug, Deserialize)]
struct CreateCertResponse {
    #[serde(rename = "privateKey")]
    private_key: PrivateKey,
    certificate: String,
    #[allow(dead_code)]
    expiration: String,
}

#[derive(Serialize)]
struct HeaderKeyValue {
    name: String,
    value: String,
}

#[derive(Serialize)]
struct PublishEndpoint {
    url: String,
    headers: Vec<HeaderKeyValue>,
}

#[derive(Serialize)]
struct PublishIdEndpoint {
    id: &'static str,
    endpoint: PublishEndpoint,
}

const CERT_PATH: &str = "/cert/cert.pem";
const KEY_PATH: &str = "/cert/key.pem";

#[actix_web::main]
async fn main() {
    log_panics::init();

    let mut builder = if cfg!(debug_assertions) {
        Builder::from_env(Env::default().default_filter_or("debug"))
    } else {
        Builder::from_env(Env::default().default_filter_or("info"))
    };

    builder.format(|f, record| match record.level() {
        log::Level::Error => {
            eprintln!("{}", record.args());
            Ok(())
        }
        _ => {
            writeln!(f, "{}", record.args())
        }
    });

    builder.target(Target::Stdout).init();

    info!(
        "module version: {} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_SHORT_REV")
    );

    let ui_port = std::env::var("UI_PORT")
        .expect("UI_PORT missing")
        .parse::<u64>()
        .expect("UI_PORT format");

    create_module_certificate().await;

    let mut tls_certs =
        std::io::BufReader::new(std::fs::File::open(CERT_PATH).expect("read certs_file"));
    let mut tls_key =
        std::io::BufReader::new(std::fs::File::open(KEY_PATH).expect("read key_file"));

    let tls_certs = rustls_pemfile::certs(&mut tls_certs)
        .collect::<Result<Vec<_>, _>>()
        .expect("failed to parse cert pem");

    // set up TLS config options
    let tls_config = match rustls_pemfile::read_one(&mut tls_key)
        .expect("cannot read key pem file")
        .expect("nothing found in key pem file")
    {
        rustls_pemfile::Item::Pkcs1Key(key) => rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(tls_certs, rustls::pki_types::PrivateKeyDer::Pkcs1(key))
            .expect("invalid tls config"),
        rustls_pemfile::Item::Pkcs8Key(key) => rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(tls_certs, rustls::pki_types::PrivateKeyDer::Pkcs8(key))
            .expect("invalid tls config"),
        _ => panic!("unexpected item found in key pem file"),
    };

    fs::exists("/data").expect("data dir /data is missing");

    fn session_middleware() -> SessionMiddleware<CookieSessionStore> {
        SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64]))
            .cookie_name(String::from("omnect-ui-session"))
            .cookie_secure(true)
            .session_lifecycle(BrowserSession::default())
            .cookie_same_site(SameSite::Strict)
            .cookie_content_security(CookieContentSecurity::Private)
            .cookie_http_only(true)
            .build()
    }

    let centrifugo_client_token_hmac_secret_key = Uuid::new_v4().to_string();
    let centrifugo_http_api_key = Uuid::new_v4().to_string();

    std::env::set_var(
        "CENTRIFUGO_CLIENT_TOKEN_HMAC_SECRET_KEY",
        &centrifugo_client_token_hmac_secret_key,
    );
    std::env::set_var("CENTRIFUGO_HTTP_API_KEY", &centrifugo_http_api_key);
    std::env::set_var(
        "CENTRIFUGO_HTTP_SERVER_PORT",
        &centrifugo_http_server_port!(),
    );

    let ods_socket_path = std::env::var("SOCKET_PATH").expect("env SOCKET_PATH is missing");
    let index_html =
        std::fs::canonicalize("static/index.html").expect("static/index.html not found");

    fs::exists(&ods_socket_path).unwrap_or_else(|_| {
        panic!(
            "omnect device service socket file {} does not exist",
            &ods_socket_path
        )
    });

    fs::exists(&update_os_path!())
        .unwrap_or_else(|_| panic!("path {} for os update does not exist", &update_os_path!()));

    send_publish_endpoint(&centrifugo_http_api_key, &ods_socket_path).await;

    let api_config = Api {
        ods_socket_path: ods_socket_path.clone(),
        update_os_path: update_os_path!(),
        centrifugo_client_token_hmac_secret_key,
        index_html,
    };

    let server = HttpServer::new(move || {
        App::new()
            .wrap(session_middleware())
            .app_data(
                MultipartFormConfig::default()
                    .total_limit(UPLOAD_LIMIT_BYTES)
                    .memory_limit(MEMORY_LIMIT_BYTES),
            )
            .app_data(Data::new(api_config.clone()))
            .route("/", web::get().to(Api::index))
            .route(
                "/factory-reset",
                web::post().to(Api::factory_reset).wrap(middleware::AuthMw),
            )
            .route(
                "/reboot",
                web::post().to(Api::reboot).wrap(middleware::AuthMw),
            )
            .route(
                "/reload-network",
                web::post().to(Api::reload_network).wrap(middleware::AuthMw),
            )
            .route(
                "/update/file",
                web::post().to(Api::save_file).wrap(middleware::AuthMw),
            )
            .route(
                "/update/load",
                web::post().to(Api::load_update).wrap(middleware::AuthMw),
            )
            .route(
                "/update/run",
                web::post().to(Api::run_update).wrap(middleware::AuthMw),
            )
            .route(
                "/token/login",
                web::post().to(Api::token).wrap(middleware::AuthMw),
            )
            .route(
                "/token/refresh",
                web::get().to(Api::token).wrap(middleware::AuthMw),
            )
            .route(
                "/require-set-password",
                web::get().to(Api::require_set_password),
            )
            .route("/set-password", web::post().to(Api::set_password))
            .route("/update-password", web::post().to(Api::update_password))
            .route("/version", web::get().to(Api::version))
            .route("/logout", web::post().to(Api::logout))
            .service(Files::new(
                "/static",
                std::fs::canonicalize("static").expect("static folder not found"),
            ))
            .default_service(web::route().to(Api::index))
    })
    .bind_rustls_0_23(format!("0.0.0.0:{ui_port}"), tls_config)
    .expect("bind_rustls")
    .disable_signals()
    .run();

    let server_handle = server.handle();
    let server_task = tokio::spawn(server);

    std::env::set_var("CENTRIFUGO_HTTP_SERVER_TLS_CERT_PEM", CERT_PATH);
    std::env::set_var("CENTRIFUGO_HTTP_SERVER_TLS_KEY_PEM", KEY_PATH);

    let mut centrifugo =
        Command::new(std::fs::canonicalize("centrifugo").expect("centrifugo not found"))
            .arg("-c")
            .arg("/centrifugo_config.json")
            .spawn()
            .expect("Failed to spawn child process");

    debug!("centrifugo pid: {}", centrifugo.id().unwrap());

    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            debug!("ctrl-c");
            delete_publish_endpoint(&ods_socket_path).await;
            server_handle.stop(true).await;
        },
        _ = sigterm.recv() => {
            debug!("SIGTERM received");
            delete_publish_endpoint(&ods_socket_path).await;
            server_handle.stop(true).await;
        },
        _ = server_task => {
            debug!("server stopped");
            centrifugo.kill().await.expect("kill centrifugo failed");
            debug!("centrifugo killed");
        },
        _ = centrifugo.wait() => {
            debug!("centrifugo stopped");
            server_handle.stop(true).await;
            debug!("server stopped");
        }
    }

    debug!("good bye");
}

#[cfg(feature = "mock")]
async fn create_module_certificate() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[cfg(not(feature = "mock"))]
async fn create_module_certificate() -> impl Responder {
    info!("create_module_certificate()");

    let iotedge_moduleid = std::env::var("IOTEDGE_MODULEID").expect("IOTEDGE_MODULEID missing");
    let iotedge_deviceid = std::env::var("IOTEDGE_DEVICEID").expect("IOTEDGE_DEVICEID missing");
    let iotedge_modulegenerationid =
        std::env::var("IOTEDGE_MODULEGENERATIONID").expect("IOTEDGE_MODULEGENERATIONID missing");
    let iotedge_apiversion =
        std::env::var("IOTEDGE_APIVERSION").expect("IOTEDGE_APIVERSION missing");

    let iotedge_workloaduri =
        std::env::var("IOTEDGE_WORKLOADURI").expect("IOTEDGE_WORKLOADURI missing");

    let payload = CreateCertPayload {
        common_name: iotedge_deviceid.to_string(),
    };
    let path = format!(
        "/modules/{}/genid/{}/certificate/server?api-version={}",
        iotedge_moduleid, iotedge_modulegenerationid, iotedge_apiversion
    );
    let ori_socket_path = iotedge_workloaduri.to_string();
    let socket_path = ori_socket_path.strip_prefix("unix://").unwrap();

    match socket_client::post_with_json_body(&path, payload, socket_path).await {
        Ok(response) => {
            let body = response.into_body();
            let body_bytes = body.try_into_bytes().unwrap();
            let cert_response: CreateCertResponse =
                serde_json::from_slice(&body_bytes).expect("CreateCertResponse not possible");

            let mut file = File::create(CERT_PATH)
                .unwrap_or_else(|_| panic!("{CERT_PATH} could not be created"));
            file.write_all(cert_response.certificate.as_bytes())
                .unwrap_or_else(|_| panic!("write to {CERT_PATH} not possible"));

            let mut file = File::create(KEY_PATH)
                .unwrap_or_else(|_| panic!("{KEY_PATH} could not be created"));
            file.write_all(cert_response.private_key.bytes.as_bytes())
                .unwrap_or_else(|_| panic!("write to {KEY_PATH} not possible"));

            HttpResponse::Ok().finish()
        }
        Err(e) => {
            error!("create_module_certificate failed: {e:#}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn send_publish_endpoint(
    centrifugo_http_api_key: &str,
    ods_socket_path: &str,
) -> impl Responder {
    let headers = vec![
        HeaderKeyValue {
            name: String::from("Content-Type"),
            value: String::from("application/json"),
        },
        HeaderKeyValue {
            name: String::from("X-API-Key"),
            value: String::from(centrifugo_http_api_key),
        },
    ];

    let body = PublishIdEndpoint {
        id: env!("CARGO_PKG_NAME"),
        endpoint: PublishEndpoint {
            url: format!(
                "https://localhost:{}/api/publish",
                &centrifugo_http_server_port!()
            ),
            headers,
        },
    };

    if let Err(e) =
        socket_client::post_with_json_body("/publish-endpoint/v1", body, ods_socket_path).await
    {
        error!("sending publish endpoint failed: {e:#}");
        HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

async fn delete_publish_endpoint(ods_socket_path: &str) -> impl Responder {
    static ENDPOINT: &str = concat!("/publish-endpoint/v1/", env!("CARGO_PKG_NAME"));

    if let Err(e) = socket_client::delete_with_empty_body(ENDPOINT, ods_socket_path).await {
        error!("deleting publish endpoint failed: {e:#}");
        HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

pub fn validate_password(password: &str) -> Result<()> {
    if password.is_empty() {
        error!("password is empty");
        bail!("password is empty");
    }

    let password_file = config_path!("password");

    let Ok(password_hash) = fs::read_to_string(password_file) else {
        error!("failed to read password file");
        bail!("failed to read password file");
    };

    if password_hash.is_empty() {
        error!("password hash is empty");
        bail!("password hash is empty");
    }

    let Ok(parsed_hash) = PasswordHash::new(&password_hash) else {
        error!("failed to parse password hash");
        bail!("failed to parse password hash");
    };

    if let Err(e) = Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
        error!("password verification failed: {e:#}");
        bail!("password verification failed");
    }

    Ok(())
}
