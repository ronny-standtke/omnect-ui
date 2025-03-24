use actix_files::{Files, NamedFile};
use actix_session::{
    config::{BrowserSession, CookieContentSecurity},
    storage::CookieSessionStore,
    Session, SessionMiddleware,
};
use actix_web::{
    body::MessageBody,
    cookie::{Key, SameSite},
    http::StatusCode,
    web::{self},
    App, HttpResponse, HttpServer, Responder,
};
use anyhow::{Context, Result};
use env_logger::{Builder, Env, Target};
use http_body_util::BodyExt;
use hyper::{client::conn::http1, Request};
use hyper_util::rt::TokioIo;
use jwt_simple::prelude::*;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::sync::LazyLock;
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

static ODS_SOCKET_PATH: LazyLock<String> =
    LazyLock::new(|| std::env::var("SOCKET_PATH").expect("SOCKET_PATH missing"));

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

    let server = HttpServer::new(move || {
        App::new()
            .route("/", web::get().to(index))
            .route(
                "/factory-reset",
                web::post().to(factory_reset).wrap(middleware::AuthMw),
            )
            .route("/reboot", web::post().to(reboot).wrap(middleware::AuthMw))
            .route(
                "/reload-network",
                web::post().to(reload_network).wrap(middleware::AuthMw),
            )
            .route(
                "/token/login",
                web::post().to(token).wrap(middleware::AuthMw),
            )
            .route(
                "/token/refresh",
                web::get().to(token).wrap(middleware::AuthMw),
            )
            .route("/logout", web::post().to(logout))
            .service(web::redirect("/login", "/"))
            .service(Files::new(
                "/static",
                std::fs::canonicalize("static").expect("static folder not found"),
            ))
            .wrap(session_middleware())
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

    match post_with_json_body(&path, Some(payload), socket_path).await {
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

async fn index() -> actix_web::Result<NamedFile> {
    debug!("index() called");

    // trigger omnect-device-service to republish
    match post_with_empty_body("/republish/v1", &ODS_SOCKET_PATH).await {
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

    match post_with_json_body("/factory-reset/v1", Some(payload), &ODS_SOCKET_PATH).await {
        Ok(response) => response,
        Err(e) => {
            error!("factory_reset failed: {e:#}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn reboot() -> impl Responder {
    debug!("reboot() called");

    match post_with_empty_body("/reboot/v1", &ODS_SOCKET_PATH).await {
        Ok(response) => response,
        Err(e) => {
            error!("reboot failed: {e:#}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn reload_network() -> impl Responder {
    debug!("reload_network() called");

    match post_with_empty_body("/reload-network/v1", &ODS_SOCKET_PATH).await {
        Ok(response) => response,
        Err(e) => {
            error!("reload-network failed: {e:#}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn post_with_json_body(
    path: &str,
    body: impl Serialize,
    socket_path: &str,
) -> Result<HttpResponse> {
    let json = match serde_json::to_value(body) {
        Ok(r) => r,
        Err(e) => {
            error!("failed to serialize data error: {e:#}");
            return Ok(HttpResponse::InternalServerError().finish());
        }
    };

    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Host", "localhost")
        .body(serde_json::to_string(&json).unwrap_or_default())
        .context("build request failed")?;

    post(request, socket_path).await
}

async fn post_with_empty_body(path: &str, socket_path: &str) -> Result<HttpResponse> {
    let request = Request::builder()
        .uri(path)
        .method("POST")
        .header("Host", "localhost")
        .body(String::new())
        .context("build request failed")?;

    post(request, socket_path).await
}

async fn post(request: Request<String>, socket_path: &str) -> Result<HttpResponse> {
    let mut sender = match sender(socket_path).await {
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

async fn sender(socket_path: &str) -> Result<http1::SendRequest<String>> {
    let stream = UnixStream::connect(socket_path)
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

async fn token(session: Session) -> impl Responder {
    if let Ok(key) = std::env::var("CENTRIFUGO_CLIENT_TOKEN_HMAC_SECRET_KEY") {
        let key = HS256Key::from_bytes(key.as_bytes());
        let claims = Claims::create(Duration::from_hours(middleware::TOKEN_EXPIRE_HOURS))
            .with_subject("omnect-ui");

        if let Ok(token) = key.authenticate(claims) {
            match session.insert("token", token.clone()) {
                Ok(_) => return HttpResponse::Ok().body(token),
                Err(_) => return HttpResponse::InternalServerError().body("Error."),
            }
        } else {
            error!("token: cannot create token");
        };
    } else {
        error!("token: missing secret key");
    };

    HttpResponse::InternalServerError().finish()
}

async fn logout(session: Session) -> impl Responder {
    debug!("logout() called");
    session.purge();
    HttpResponse::Ok()
}
