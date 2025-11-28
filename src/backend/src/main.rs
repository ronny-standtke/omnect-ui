mod api;
mod config;
mod http_client;
mod keycloak_client;
mod middleware;
mod omnect_device_service_client;
mod services;

use crate::{
    api::Api,
    config::AppConfig,
    keycloak_client::KeycloakProvider,
    omnect_device_service_client::{DeviceServiceClient, OmnectDeviceServiceClient},
    services::{
        auth::TokenManager,
        certificate::{CertificateService, CreateCertPayload},
        network::NetworkConfigService,
    },
};
use actix_cors::Cors;
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
use actix_web_static_files::ResourceFiles;
use anyhow::{Context, Result};
use env_logger::{Builder, Env, Target};
use log::{debug, error, info, warn};
use rustls::crypto::{CryptoProvider, ring::default_provider};
use std::io::Write;
use tokio::{
    process::{Child, Command},
    signal::unix::{SignalKind, signal},
    sync::broadcast,
};

const UPLOAD_LIMIT_BYTES: usize = 250 * 1024 * 1024;
const MEMORY_LIMIT_BYTES: usize = 10 * 1024 * 1024;

// Include the generated static files from build.rs
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

// Alias the generated function to a more descriptive name
#[inline(always)]
fn static_files() -> std::collections::HashMap<&'static str, static_files::Resource> {
    generate()
}

type UiApi = Api<OmnectDeviceServiceClient, KeycloakProvider>;

enum ShutdownReason {
    Restart,
    Shutdown,
}

impl std::fmt::Display for ShutdownReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShutdownReason::Restart => write!(f, "restarting server"),
            ShutdownReason::Shutdown => write!(f, "shutting down"),
        }
    }
}

#[actix_web::main]
async fn main() {
    if let Err(e) = run().await {
        error!("application error: {e:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    initialize()?;

    let mut restart_rx = NetworkConfigService::setup_restart_receiver()
        .map_err(|_| anyhow::anyhow!("restart receiver already initialized"))?;

    let mut sigterm =
        signal(SignalKind::terminate()).context("failed to install SIGTERM handler")?;

    let mut service_client =
        OmnectDeviceServiceClient::new().context("failed to create device service client")?;

    while let ShutdownReason::Restart =
        run_until_shutdown(&mut service_client, &mut restart_rx, &mut sigterm).await?
    {}

    Ok(())
}

fn initialize() -> Result<()> {
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

    CryptoProvider::install_default(default_provider())
        .map_err(|_| anyhow::anyhow!("crypto provider already installed"))?;

    KeycloakProvider::create_frontend_config_file()
        .context("failed to create frontend config file")?;

    if NetworkConfigService::rollback_exists() {
        warn!("unexpectedly started with pending network rollback");
    }

    Ok(())
}

async fn run_until_shutdown(
    service_client: &mut OmnectDeviceServiceClient,
    restart_rx: &mut broadcast::Receiver<()>,
    sigterm: &mut tokio::signal::unix::Signal,
) -> Result<ShutdownReason> {
    info!("starting server");

    // 1. create the cert with the ip in CommonName
    let common_name = service_client
        .ip_address()
        .await
        .context("failed to get IP address")?;

    CertificateService::create_module_certificate(CreateCertPayload { common_name })
        .await
        .context("failed to create certificate")?;

    // 2. run centrifugo with valid cert
    let mut centrifugo = run_centrifugo().context("failed to start centrifugo")?;

    // 3. register publish endpoint with running centrifugo
    if !service_client.has_publish_endpoint {
        service_client
            .register_publish_endpoint(AppConfig::get().centrifugo.publish_endpoint.clone())
            .await
            .context("failed to register publish endpoint")?;
    }

    let (server_handle, server_task) = run_server(service_client.clone()).await?;

    if let Err(e) = NetworkConfigService::process_pending_rollback(service_client).await {
        error!("failed to process pending rollback: {e:#}");
    }

    let reason = tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            debug!("ctrl-c received");
            ShutdownReason::Shutdown
        },
        _ = sigterm.recv() => {
            debug!("SIGTERM received");
            ShutdownReason::Shutdown
        },
        _ = restart_rx.recv() => {
            debug!("server restart requested");
            ShutdownReason::Restart
        },
        result = server_task => {
            match result {
                Ok(Ok(())) => debug!("server stopped normally"),
                Ok(Err(e)) => debug!("server stopped with error: {e}"),
                Err(e) => debug!("server task panicked: {e}"),
            }
            ShutdownReason::Shutdown
        },
        _ = centrifugo.wait() => {
            debug!("centrifugo stopped unexpectedly");
            ShutdownReason::Shutdown
        }
    };

    info!("{reason}");

    server_handle.stop(true).await;
    if let Err(e) = centrifugo.kill().await {
        error!("failed to kill centrifugo: {e:#}");
    }

    if matches!(reason, ShutdownReason::Shutdown) {
        if let Err(e) = service_client.shutdown().await {
            error!("failed to shutdown service client: {e:#}");
        }
        info!("shutdown complete");
    }

    Ok(reason)
}

async fn run_server(
    service_client: OmnectDeviceServiceClient,
) -> Result<(
    ServerHandle,
    tokio::task::JoinHandle<Result<(), std::io::Error>>,
)> {
    let api = UiApi::new(service_client.clone(), Default::default())
        .await
        .context("failed to create api")?;

    let tls_config = load_tls_config().context("failed to load tls config")?;
    let config = &AppConfig::get();
    let ui_port = config.ui.port;
    let session_key = Key::generate();
    let token_manager = TokenManager::new(&config.centrifugo.client_token);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_header()
                    .allowed_methods(vec!["GET"])
                    .supports_credentials()
                    .max_age(3600),
            )
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
            .app_data(Data::new(token_manager.clone()))
            .app_data(Data::new(api.clone()))
            .app_data(Data::new(static_files()))
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
                web::post()
                    .to(UiApi::upload_firmware_file)
                    .wrap(middleware::AuthMw),
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
            .route("/network", web::post().to(UiApi::set_network_config))
            .service(ResourceFiles::new("/static", static_files()))
            .default_service(web::route().to(UiApi::index))
    })
    .bind_rustls_0_23(format!("0.0.0.0:{ui_port}"), tls_config)
    .context("failed to bind server")?
    .disable_signals()
    .run();

    Ok((server.handle(), tokio::spawn(server)))
}

fn run_centrifugo() -> Result<Child> {
    let config = &AppConfig::get().centrifugo;
    let certificate = &AppConfig::get().certificate;

    let centrifugo = Command::new(&config.binary_path)
        .arg("-c")
        .arg("/centrifugo_config.json")
        .envs(vec![
            (
                "CENTRIFUGO_HTTP_SERVER_TLS_CERT_PEM",
                certificate.cert_path.to_string_lossy().to_string(),
            ),
            (
                "CENTRIFUGO_HTTP_SERVER_TLS_KEY_PEM",
                certificate.key_path.to_string_lossy().to_string(),
            ),
            ("CENTRIFUGO_HTTP_SERVER_PORT", config.port.clone()),
            (
                "CENTRIFUGO_CLIENT_TOKEN_HMAC_SECRET_KEY",
                config.client_token.clone(),
            ),
            ("CENTRIFUGO_HTTP_API_KEY", config.api_key.clone()),
            ("CENTRIFUGO_LOG_LEVEL", config.log_level.clone()),
        ])
        .spawn()
        .context("failed to spawn centrifugo process")?;

    info!(
        "centrifugo pid: {}",
        centrifugo
            .id()
            .context("failed to get centrifugo process id")?
    );

    Ok(centrifugo)
}

fn load_tls_config() -> Result<rustls::ServerConfig> {
    let paths = &AppConfig::get().certificate;

    let mut tls_certs = std::io::BufReader::new(
        std::fs::File::open(&paths.cert_path).context("failed to open certificate file")?,
    );

    let mut tls_key = std::io::BufReader::new(
        std::fs::File::open(&paths.key_path).context("failed to open key file")?,
    );

    let tls_certs = rustls_pemfile::certs(&mut tls_certs)
        .collect::<Result<Vec<_>, _>>()
        .context("failed to parse certificate pem")?;

    let key_item = rustls_pemfile::read_one(&mut tls_key)
        .context("failed to read key pem file")?
        .context("no valid key found in pem file")?;

    let config = match key_item {
        rustls_pemfile::Item::Pkcs1Key(key) => rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(tls_certs, rustls::pki_types::PrivateKeyDer::Pkcs1(key))
            .context("failed to create tls config with pkcs1 key")?,
        rustls_pemfile::Item::Pkcs8Key(key) => rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(tls_certs, rustls::pki_types::PrivateKeyDer::Pkcs8(key))
            .context("failed to create tls config with pkcs8 key")?,
        _ => anyhow::bail!("unexpected key type in pem file"),
    };

    Ok(config)
}
