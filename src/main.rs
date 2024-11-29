use actix_files::{Files, NamedFile};
use actix_web::{http::StatusCode, web, App, HttpResponse, HttpServer, Responder};
use anyhow::{Context, Result};
use env_logger::{Builder, Env, Target};
use http_body_util::BodyExt;
use hyper::{client::conn::http1, Request};
use hyper_util::rt::TokioIo;
use jwt_simple::prelude::*;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::io::Write;
use tokio::{net::UnixStream, process::Command};

mod middleware;

#[derive(Deserialize)]
struct FactoryResetInput {
    preserve: Vec<String>,
}

#[derive(Serialize)]
struct FactoryResetPayload {
    mode: u8,
    preserve: Vec<String>,
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

    info!("module version: {}", env!("CARGO_PKG_VERSION"));

    let ui_port = std::env::var("UI_PORT")
        .expect("UI_PORT missing")
        .parse::<u64>()
        .expect("UI_PORT format");

    let device_cert_path = std::env::var("SSL_CERT_PATH").expect("SSL_CERT_PATH missing");
    let device_key_path = std::env::var("SSL_KEY_PATH").expect("SSL_KEY_PATH missing");

    info!("device cert file: {device_cert_path}");
    info!("device key file: {device_key_path}");

    let mut tls_certs =
        std::io::BufReader::new(std::fs::File::open(device_cert_path).expect("read certs_file"));
    let mut tls_key =
        std::io::BufReader::new(std::fs::File::open(device_key_path).expect("read key_file"));

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

    let server = HttpServer::new(move || {
        App::new()
            .route("/", web::get().to(index))
            .route(
                "/factory-reset",
                web::post().to(factory_reset).wrap(middleware::BearerAuthMw),
            )
            .route(
                "/reboot",
                web::post().to(reboot).wrap(middleware::BearerAuthMw),
            )
            .route(
                "/reload-network",
                web::post()
                    .to(reload_network)
                    .wrap(middleware::BearerAuthMw),
            )
            .route(
                "/token/login",
                web::post().to(token).wrap(middleware::BasicAuthMw),
            )
            .route(
                "/token/refresh",
                web::get().to(token).wrap(middleware::BearerAuthMw),
            )
            .service(Files::new(
                "/static",
                std::fs::canonicalize("static").expect("static folder not found"),
            ))
    })
    .bind_rustls_0_22(format!("0.0.0.0:{ui_port}"), tls_config)
    .expect("bind_rustls")
    .disable_signals()
    .run();

    let server_handle = server.handle();
    let server_task = tokio::spawn(server);

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

async fn index() -> actix_web::Result<NamedFile> {
    debug!("index() called");

    // trigger omnect-device-service to republish
    match post_with_empty_body("/republish/v1").await {
        Ok(response) => response,
        Err(e) => {
            error!("republish failed: {e:#}");
            return Err(actix_web::error::ErrorInternalServerError(
                "republish failed",
            ));
        }
    };

    Ok(NamedFile::open(
        std::fs::canonicalize("static/index.html").expect("static/index.html not found"),
    )?)
}

async fn factory_reset(body: web::Json<FactoryResetInput>) -> impl Responder {
    debug!(
        "factory_reset() called with preserved keys {}",
        body.preserve.join(",")
    );

    let payload = FactoryResetPayload {
        mode: 1,
        preserve: body.preserve.clone(),
    };

    match post_with_json_body("/factory-reset/v1", Some(payload)).await {
        Ok(response) => response,
        Err(e) => {
            error!("factory_reset failed: {e:#}");
            HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).finish()
        }
    }
}

async fn reboot() -> impl Responder {
    debug!("reboot() called");

    match post_with_empty_body("/reboot/v1").await {
        Ok(response) => response,
        Err(e) => {
            error!("reboot failed: {e:#}");
            HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).finish()
        }
    }
}

async fn reload_network() -> impl Responder {
    debug!("reload_network() called");

    match post_with_empty_body("/reload-network/v1").await {
        Ok(response) => response,
        Err(e) => {
            error!("reload-network failed: {e:#}");
            HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).finish()
        }
    }
}

async fn post_with_json_body(path: &str, body: impl Serialize) -> Result<HttpResponse> {
    let json = match serde_json::to_value(body) {
        Ok(r) => r,
        Err(e) => {
            error!("failed to serialize data error: {e:#}");
            return Ok(HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).finish());
        }
    };

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Host", "localhost")
        .body(serde_json::to_string(&json).unwrap_or_default())
        .context("build request failed")?;

    post(request).await
}

async fn post_with_empty_body(path: &str) -> Result<HttpResponse> {
    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Host", "localhost")
        .body(String::new())
        .context("build request failed")?;

    post(request).await
}

async fn post(request: Request<String>) -> Result<HttpResponse> {
    let mut sender = match sender().await {
        Err(e) => {
            error!("error creating request sender: {e}. socket might be broken. exit application");
            std::process::exit(1)
        }
        Ok(sender) => sender,
    };

    let res = sender
        .send_request(request)
        .await
        .context("send request failed")?;

    let status_code =
        StatusCode::from_u16(res.status().as_u16()).context("get status code failed")?;

    let body = res
        .collect()
        .await
        .context("collect response body failed")?;

    let body = String::from_utf8(body.to_bytes().to_vec()).context("get response body failed")?;

    Ok(HttpResponse::build(status_code).body(body))
}

async fn sender() -> Result<http1::SendRequest<String>> {
    let stream = UnixStream::connect(std::env::var("SOCKET_PATH").expect("SOCKET_PATH missing"))
        .await
        .context("cannot create unix stream")?;

    let (mut sender, conn) = http1::handshake(TokioIo::new(stream))
        .await
        .context("unix stream handshake failed")?;

    actix_rt::spawn(async move {
        if let Err(err) = conn.await {
            error!("post connection failed: {:?}", err);
        }
    });

    sender
        .ready()
        .await
        .context("unix stream unexpectedly closed")?;

    Ok(sender)
}

async fn token() -> impl Responder {
    if let Ok(key) = std::env::var("CENTRIFUGO_TOKEN_HMAC_SECRET_KEY") {
        let key = HS256Key::from_bytes(key.as_bytes());
        let claims = Claims::create(Duration::from_hours(middleware::TOKEN_EXPIRE_HOURS))
            .with_subject("omnect-ui");

        if let Ok(token) = key.authenticate(claims) {
            return HttpResponse::Ok().body(token);
        } else {
            error!("token: cannot create token");
        };
    } else {
        error!("token: missing secret key");
    };

    HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).finish()
}
