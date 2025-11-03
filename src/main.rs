mod api;
mod certificate;
mod common;
mod keycloak_client;
mod middleware;
mod omnect_device_service_client;

use crate::{
    api::Api,
    certificate::create_module_certificate,
    omnect_device_service_client::{DeviceServiceClient, OmnectDeviceServiceClient},
};
use actix_files::Files;
use actix_multipart::form::MultipartFormConfig;
use actix_server::ServerHandle;
use actix_session::{
    SessionMiddleware,
    config::{BrowserSession, CookieContentSecurity},
    storage::CookieSessionStore,
};
use actix_web::{
    App, HttpServer,
    cookie::{Key, SameSite},
    web::{self, Data},
};
use anyhow::Result;
use common::{centrifugo_config, config_path};
use env_logger::{Builder, Env, Target};
use keycloak_client::KeycloakProvider;
use log::{debug, error, info};
use rustls::crypto::{CryptoProvider, ring::default_provider};
use std::{fs, io::Write};
use tokio::{
    process::{Child, Command},
    signal::unix::{SignalKind, signal},
};

const UPLOAD_LIMIT_BYTES: usize = 250 * 1024 * 1024;
const MEMORY_LIMIT_BYTES: usize = 10 * 1024 * 1024;

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

    create_module_certificate()
        .await
        .expect("failed to create module certificate");

    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
    let mut centrifugo = run_centrifugo();
    let (server_handle, server_task, service_client) = run_server().await;

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            debug!("ctrl-c");
            server_handle.stop(true).await;
        },
        _ = sigterm.recv() => {
            debug!("SIGTERM received");
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

    // Shutdown service client
    if let Err(e) = service_client.shutdown().await {
        error!("failed to shutdown service client: {}", e);
    }

    debug!("good bye");
}

async fn run_server() -> (
    ServerHandle,
    tokio::task::JoinHandle<Result<(), std::io::Error>>,
    OmnectDeviceServiceClient,
) {
    CryptoProvider::install_default(default_provider()).expect("failed to install crypto provider");

    let Ok(true) = fs::exists("/data") else {
        panic!("data dir /data is missing");
    };

    if !fs::exists(config_path!()).is_ok_and(|ok| ok) {
        fs::create_dir_all(config_path!()).expect("failed to create config directory");
    };

    common::create_frontend_config_file().expect("failed to create frontend config file");

    type UiApi = Api<OmnectDeviceServiceClient, KeycloakProvider>;

    let service_client = OmnectDeviceServiceClient::new(true)
        .await
        .expect("failed to create client to device service");

    let api = UiApi::new(service_client.clone(), Default::default())
        .await
        .expect("failed to create api");

    let mut tls_certs = std::io::BufReader::new(
        std::fs::File::open(certificate::cert_path()).expect("read certs_file"),
    );
    let mut tls_key = std::io::BufReader::new(
        std::fs::File::open(certificate::key_path()).expect("read key_file"),
    );

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

    let ui_port = std::env::var("UI_PORT")
        .expect("UI_PORT missing")
        .parse::<u64>()
        .expect("UI_PORT format");

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
            .app_data(Data::new(api.clone()))
            .route("/", web::get().to(UiApi::index))
            .route("/config.js", web::get().to(UiApi::config))
            .route(
                "/factory-reset",
                web::post()
                    .to(UiApi::factory_reset)
                    .wrap(middleware::AuthMw),
            )
            .route(
                "/reboot",
                web::post().to(UiApi::reboot).wrap(middleware::AuthMw),
            )
            .route(
                "/reload-network",
                web::post()
                    .to(UiApi::reload_network)
                    .wrap(middleware::AuthMw),
            )
            .route(
                "/update/file",
                web::post().to(UiApi::save_file).wrap(middleware::AuthMw),
            )
            .route(
                "/update/load",
                web::post().to(UiApi::load_update).wrap(middleware::AuthMw),
            )
            .route(
                "/update/run",
                web::post().to(UiApi::run_update).wrap(middleware::AuthMw),
            )
            .route(
                "/token/login",
                web::post().to(UiApi::token).wrap(middleware::AuthMw),
            )
            .route(
                "/token/refresh",
                web::get().to(UiApi::token).wrap(middleware::AuthMw),
            )
            .route(
                "/token/validate",
                web::post().to(UiApi::validate_portal_token),
            )
            .route(
                "/require-set-password",
                web::get().to(UiApi::require_set_password),
            )
            .route("/set-password", web::post().to(UiApi::set_password))
            .route("/update-password", web::post().to(UiApi::update_password))
            .route("/version", web::get().to(UiApi::version))
            .route("/logout", web::post().to(UiApi::logout))
            .route("/healthcheck", web::get().to(UiApi::healthcheck))
            .service(Files::new(
                "/static",
                std::fs::canonicalize("static").expect("static folder not found"),
            ))
            .default_service(web::route().to(UiApi::index))
    })
    .bind_rustls_0_23(format!("0.0.0.0:{ui_port}"), tls_config)
    .expect("bind_rustls")
    .disable_signals()
    .run();

    (server.handle(), tokio::spawn(server), service_client)
}

fn run_centrifugo() -> Child {
    let centrifugo =
        Command::new(std::fs::canonicalize("centrifugo").expect("centrifugo not found"))
            .arg("-c")
            .arg("/centrifugo_config.json")
            .envs(vec![
                (
                    "CENTRIFUGO_HTTP_SERVER_TLS_CERT_PEM",
                    certificate::cert_path(),
                ),
                (
                    "CENTRIFUGO_HTTP_SERVER_TLS_KEY_PEM",
                    certificate::key_path(),
                ),
                ("CENTRIFUGO_HTTP_SERVER_PORT", centrifugo_config().port),
                (
                    "CENTRIFUGO_CLIENT_TOKEN_HMAC_SECRET_KEY",
                    centrifugo_config().client_token,
                ),
                ("CENTRIFUGO_HTTP_API_KEY", centrifugo_config().api_key),
            ])
            .spawn()
            .expect("Failed to spawn child process");

    info!(
        "centrifugo pid: {}",
        centrifugo.id().expect("centrifugo pid")
    );

    centrifugo
}
