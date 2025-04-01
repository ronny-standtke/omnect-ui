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
    web::{self},
    App, HttpResponse, HttpServer, Responder,
};
use anyhow::Result;
use env_logger::{Builder, Env, Target};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::Write;
use tokio::process::Command;

const UPLOAD_LIMIT_BYTES: usize = 250 * 1024 * 1024;
const MEMORY_LIMIT_BYTES: usize = 10 * 1024 * 1024;

macro_rules! update_os_path {
    () => {{
        static DATA_DIR_PATH_DEFAULT: &'static str = "/var/lib/omnect-ui";
        std::env::var("DATA_DIR_PATH").unwrap_or(DATA_DIR_PATH_DEFAULT.to_string())
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

    info!("module version: {}", env!("CARGO_PKG_VERSION"));

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

    let ods_socket_path = std::env::var("SOCKET_PATH").expect("env SOCKET_PATH is missing");
    let centrifugo_client_token_hmac_secret_key =
        std::env::var("CENTRIFUGO_CLIENT_TOKEN_HMAC_SECRET_KEY").expect("missing jwt secret");
    let username = std::env::var("LOGIN_USER").expect("login_token: missing user");
    let password = std::env::var("LOGIN_PASSWORD").expect("login_token: missing password");
    let index_html =
        std::fs::canonicalize("static/index.html").expect("static/index.html not found");

    let server = HttpServer::new(move || {
        App::new()
            .wrap(session_middleware())
            .app_data(
                MultipartFormConfig::default()
                    .total_limit(UPLOAD_LIMIT_BYTES)
                    .memory_limit(MEMORY_LIMIT_BYTES),
            )
            .app_data(Api::new(
                &ods_socket_path,
                &update_os_path!(),
                &centrifugo_client_token_hmac_secret_key,
                &username,
                &password,
                &index_html.to_path_buf(),
            ))
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
            .spawn()
            .expect("Failed to spawn child process");

    debug!("centrifugo pid: {}", centrifugo.id().unwrap());

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            debug!("ctrl-c");
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
