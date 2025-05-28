mod api;
mod certificate;
mod common;
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
    cookie::{Key, SameSite},
    web::{self, Data},
    App, HttpResponse, HttpServer, Responder,
};
use anyhow::Result;
use env_logger::{Builder, Env, Target};
use log::{debug, error, info};
use rustls::crypto::{ring::default_provider, CryptoProvider};
use serde::Serialize;
use std::{fs, io::Write};
use tokio::{
    process::Command,
    signal::unix::{signal, SignalKind},
};
use uuid::Uuid;

pub const REQ_ODS_VERSION: &str = ">=0.39.0";

const UPLOAD_LIMIT_BYTES: usize = 250 * 1024 * 1024;
const MEMORY_LIMIT_BYTES: usize = 10 * 1024 * 1024;

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

macro_rules! keycloak_url {
    () => {{
        static KEYCLOAK_URL: &'static str =
            "https://keycloak.omnect.conplement.cloud/realms/cp-prod";
        std::env::var("KEYCLOAK_URL").unwrap_or(KEYCLOAK_URL.to_string())
    }};
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

macro_rules! cert_path {
    () => {{
        static CERT_PATH_DEFAULT: &'static str = "/cert/cert.pem";
        std::env::var("CERT_PATH").unwrap_or(CERT_PATH_DEFAULT.to_string())
    }};
}

macro_rules! key_path {
    () => {{
        static KEY_PATH_DEFAULT: &'static str = "/cert/key.pem";
        std::env::var("KEY_PATH").unwrap_or(KEY_PATH_DEFAULT.to_string())
    }};
}

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

    let ods_socket_path = std::env::var("SOCKET_PATH").expect("env SOCKET_PATH is missing");
    fs::exists(&ods_socket_path).unwrap_or_else(|_| {
        panic!(
            "omnect device service socket file {} does not exist",
            &ods_socket_path
        )
    });
    let version_check_result = common::check_and_store_ods_version(&ods_socket_path)
        .await
        .expect("failed to check and store ods version");

    CryptoProvider::install_default(default_provider()).expect("failed to install crypto provider");

    certificate::create_module_certificate(&cert_path!(), &key_path!())
        .await
        .expect("failed to create module certificate");

    let mut tls_certs =
        std::io::BufReader::new(std::fs::File::open(cert_path!()).expect("read certs_file"));
    let mut tls_key =
        std::io::BufReader::new(std::fs::File::open(key_path!()).expect("read key_file"));

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

    let index_html =
        std::fs::canonicalize("static/index.html").expect("static/index.html not found");

    let tenant = std::env::var("TENANT").expect("env TENANT is missing");

    fs::exists(&update_os_path!())
        .unwrap_or_else(|_| panic!("path {} for os update does not exist", &update_os_path!()));

    common::create_frontend_config_file(&keycloak_url!())
        .expect("failed to create frontend config file");

    send_publish_endpoint(&centrifugo_http_api_key, &ods_socket_path).await;

    let api_config = Api {
        ods_socket_path: ods_socket_path.clone(),
        update_os_path: update_os_path!(),
        centrifugo_client_token_hmac_secret_key,
        index_html,
        keycloak_public_key_url: keycloak_url!(),
        tenant,
        version_check_result,
    };

    let session_key = Key::generate();

    let server = HttpServer::new(move || {
        App::new()
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), session_key.clone())
                    .cookie_name(String::from("omnect-ui-session"))
                    .cookie_secure(true)
                    .session_lifecycle(BrowserSession::default())
                    .cookie_same_site(SameSite::Strict)
                    .cookie_content_security(CookieContentSecurity::Private)
                    .cookie_http_only(true)
                    .build(),
            )
            .app_data(
                MultipartFormConfig::default()
                    .total_limit(UPLOAD_LIMIT_BYTES)
                    .memory_limit(MEMORY_LIMIT_BYTES),
            )
            .app_data(Data::new(api_config.clone()))
            .route("/", web::get().to(Api::index))
            .route("/config.js", web::get().to(Api::config))
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
                "/token/validate",
                web::post().to(Api::validate_portal_token),
            )
            .route(
                "/require-set-password",
                web::get().to(Api::require_set_password),
            )
            .route("/set-password", web::post().to(Api::set_password))
            .route("/update-password", web::post().to(Api::update_password))
            .route("/version", web::get().to(Api::version))
            .route("/logout", web::post().to(Api::logout))
            .route("/healthcheck", web::get().to(Api::healthcheck))
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

    std::env::set_var("CENTRIFUGO_HTTP_SERVER_TLS_CERT_PEM", cert_path!());
    std::env::set_var("CENTRIFUGO_HTTP_SERVER_TLS_KEY_PEM", key_path!());

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
