use actix_session::SessionExt;
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    web::Data,
    Error, FromRequest, HttpMessage, HttpResponse,
};
use actix_web_httpauth::extractors::basic::BasicAuth;
use anyhow::Result;
use jwt_simple::prelude::*;
use log::error;
use std::{
    future::{ready, Future, Ready},
    pin::Pin,
    rc::Rc,
};

use crate::api::Api;
use crate::common::validate_password;

pub const TOKEN_EXPIRE_HOURS: u64 = 2;

pub struct AuthMw;

impl<S, B> Transform<S, ServiceRequest> for AuthMw
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct AuthMiddleware<S> {
    service: Rc<S>,
}

type LocalBoxFuture<T> = Pin<Box<dyn Future<Output = T> + 'static>>;

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            let Some(api_config) = req.app_data::<Data<Api>>() else {
                let http_res = HttpResponse::InternalServerError().finish();
                let (http_req, _) = req.into_parts();
                return Ok(ServiceResponse::new(http_req, http_res).map_into_right_body());
            };

            let token = match req.get_session().get::<String>("token") {
                Ok(token) => token.unwrap_or_default(),
                Err(e) => {
                    error!("failed to get session. {e:#}");
                    String::new()
                }
            };

            if !token.is_empty()
                && verify_token(&token, &api_config.centrifugo_client_token_hmac_secret_key)
            {
                let res = service.call(req).await?;
                Ok(res.map_into_left_body())
            } else {
                let mut payload = req.take_payload().take();

                let Ok(auth) = BasicAuth::from_request(req.request(), &mut payload).await else {
                    return Ok(unauthorized_error(req).map_into_right_body());
                };

                if verify_user(auth) {
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                } else {
                    Ok(unauthorized_error(req).map_into_right_body())
                }
            }
        })
    }
}

pub fn verify_token(token: &str, centrifugo_client_token_hmac_secret_key: &str) -> bool {
    let key = HS256Key::from_bytes(centrifugo_client_token_hmac_secret_key.as_bytes());
    let options = VerificationOptions {
        accept_future: true,
        time_tolerance: Some(Duration::from_mins(15)),
        max_validity: Some(Duration::from_hours(TOKEN_EXPIRE_HOURS)),
        required_subject: Some("omnect-ui".to_string()),
        ..Default::default()
    };

    key.verify_token::<NoCustomClaims>(token, Some(options))
        .is_ok()
}

fn verify_user(auth: BasicAuth) -> bool {
    let Some(password) = auth.password() else {
        return false;
    };

    if let Err(e) = validate_password(password) {
        error!("verify_user() failed: {e:#}");
        return false;
    }

    true
}

fn unauthorized_error(req: ServiceRequest) -> ServiceResponse {
    let http_res = HttpResponse::Unauthorized().finish();
    let (http_req, _) = req.into_parts();
    ServiceResponse::new(http_req, http_res)
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_http::StatusCode;
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    use std::{collections::HashMap, fs::File, io::Write, path::Path};

    use actix_session::{
        config::{BrowserSession, CookieContentSecurity},
        storage::{CookieSessionStore, SessionStore},
        SessionMiddleware,
    };
    use actix_web::{
        cookie::{Cookie, CookieJar, Key, SameSite},
        dev::ServiceResponse,
        http::header::ContentType,
        test, web, App, HttpResponse, Responder,
    };
    use actix_web_httpauth::headers::authorization::Basic;

    use base64::prelude::*;

    use jwt_simple::claims::{JWTClaims, NoCustomClaims};
    use uuid::Uuid;

    fn generate_hs256_key() -> HS256Key {
        let key_str = Uuid::new_v4().to_string();
        HS256Key::from_bytes(key_str.as_bytes())
    }

    fn generate_valid_claim() -> JWTClaims<NoCustomClaims> {
        let issued_at = Clock::now_since_epoch();
        let expires_at = issued_at
            .checked_add(Duration::from_hours(TOKEN_EXPIRE_HOURS))
            .unwrap();

        JWTClaims {
            issued_at: Some(issued_at),
            expires_at: Some(expires_at),
            invalid_before: None,
            issuer: None,
            subject: Some("omnect-ui".to_string()),
            audiences: None,
            jwt_id: None,
            nonce: None,
            custom: NoCustomClaims {},
        }
    }

    fn generate_expired_claim() -> JWTClaims<NoCustomClaims> {
        let now = Clock::now_since_epoch();
        let issued_at = now
            .checked_sub(Duration::from_hours(2 * TOKEN_EXPIRE_HOURS))
            .unwrap();
        let expires_at = now
            .checked_sub(Duration::from_hours(TOKEN_EXPIRE_HOURS))
            .unwrap();

        JWTClaims {
            issued_at: Some(issued_at),
            expires_at: Some(expires_at),
            invalid_before: None,
            issuer: None,
            subject: Some("omnect-ui".to_string()),
            audiences: None,
            jwt_id: None,
            nonce: None,
            custom: NoCustomClaims {},
        }
    }

    fn generate_invalid_subject_claim() -> JWTClaims<NoCustomClaims> {
        let issued_at = Clock::now_since_epoch();
        let expires_at = issued_at
            .checked_add(Duration::from_hours(TOKEN_EXPIRE_HOURS))
            .unwrap();

        JWTClaims {
            issued_at: Some(issued_at),
            expires_at: Some(expires_at),
            invalid_before: None,
            issuer: None,
            subject: Some("some_unknown_subject".to_string()),
            audiences: None,
            jwt_id: None,
            nonce: None,
            custom: NoCustomClaims {},
        }
    }

    fn generate_unset_subject_claim() -> JWTClaims<NoCustomClaims> {
        let issued_at = Clock::now_since_epoch();
        let expires_at = issued_at
            .checked_add(Duration::from_hours(TOKEN_EXPIRE_HOURS))
            .unwrap();

        JWTClaims {
            issued_at: Some(issued_at),
            expires_at: Some(expires_at),
            invalid_before: None,
            issuer: None,
            subject: None,
            audiences: None,
            jwt_id: None,
            nonce: None,
            custom: NoCustomClaims {},
        }
    }

    fn generate_token_and_key(claim: JWTClaims<NoCustomClaims>) -> (String, String) {
        let key = generate_hs256_key();
        let token = key.authenticate(claim).unwrap();

        (token, String::from_utf8(key.to_bytes()).unwrap())
    }

    async fn index() -> impl Responder {
        HttpResponse::Ok().body("Success")
    }

    const SESSION_SECRET: [u8; 64] = [
        0xb2, 0x64, 0x83, 0x0, 0xf5, 0xcb, 0xf6, 0x1d, 0x5c, 0x83, 0xc0, 0x90, 0x6b, 0xb2, 0xe4,
        0x26, 0x14, 0x9, 0x2b, 0xa1, 0xc4, 0xc5, 0x37, 0xe7, 0xc9, 0x20, 0x8e, 0xbc, 0xee, 0x2,
        0x3c, 0xa2, 0x32, 0x57, 0x96, 0xc9, 0x99, 0x62, 0x90, 0x4f, 0x24, 0xe5, 0x25, 0x6b, 0xe1,
        0x2b, 0x8a, 0x3, 0xa3, 0xc7, 0x1e, 0xb2, 0xb2, 0xbe, 0x29, 0x51, 0xc1, 0xe2, 0x1e, 0xb7,
        0x8, 0x15, 0xc9, 0xe0,
    ];

    async fn create_service(
        session_secret: &str,
    ) -> impl actix_service::Service<
        actix_http::Request,
        Response = ServiceResponse,
        Error = actix_web::Error,
    > {
        let key = Key::from(&SESSION_SECRET);
        let session_middleware = SessionMiddleware::builder(CookieSessionStore::default(), key)
            .cookie_name(String::from("omnect-ui-session"))
            .cookie_secure(true)
            .session_lifecycle(BrowserSession::default())
            .cookie_same_site(SameSite::Strict)
            .cookie_content_security(CookieContentSecurity::Private)
            .cookie_http_only(true)
            .build();

        let api_config = Api {
            ods_socket_path: "/some/socket/path".to_string(),
            update_os_path: "/some/update/os/path".to_string(),
            centrifugo_client_token_hmac_secret_key: session_secret.to_string(),
            index_html: Path::new("/some/index/html/path").to_path_buf(),
            keycloak_public_key_url: "https://some/keycloak/public/key/url".to_string(),
            tenant: "cp".to_string(),
        };

        test::init_service(
            App::new()
                .wrap(session_middleware)
                .app_data(Data::new(api_config.clone()))
                .route("/", web::get().to(index).wrap(AuthMw)),
        )
        .await
    }

    async fn create_cookie_for_token(token: &str) -> Cookie {
        const SESSION_ID: &str = "omnect-ui-session";
        let token_name: String = "token".to_string();

        let key = Key::from(&SESSION_SECRET);
        let mut cookie_jar = CookieJar::new();
        let mut private_jar = cookie_jar.private_mut(&key);
        let session_store = CookieSessionStore::default();

        let ttl = Clock::now_since_epoch()
            .checked_add(Duration::from_hours(2))
            .unwrap();
        let ttl = actix_web::cookie::time::Duration::seconds(ttl.as_secs().try_into().unwrap());

        let session_value = session_store
            .save(
                HashMap::from([(token_name, format!("\"{}\"", token))]),
                &ttl,
            )
            .await
            .unwrap()
            .as_ref()
            .to_string();

        private_jar.add(Cookie::new(SESSION_ID, session_value));

        cookie_jar.get(SESSION_ID).unwrap().clone()
    }

    #[tokio::test]
    async fn middleware_correct_token_should_succeed() {
        let claim = generate_valid_claim();
        let (token, session_secret) = generate_token_and_key(claim);

        let app = create_service(&session_secret).await;
        let cookie = create_cookie_for_token(&token).await;

        let req = test::TestRequest::default()
            .insert_header(ContentType::plaintext())
            .cookie(cookie)
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
    }

    #[tokio::test]
    async fn middleware_expired_token_should_require_login() {
        let claim = generate_expired_claim();
        let (token, session_secret) = generate_token_and_key(claim);

        let app = create_service(&session_secret).await;
        let cookie = create_cookie_for_token(&token).await;

        let req = test::TestRequest::default()
            .insert_header(ContentType::plaintext())
            .cookie(cookie)
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn middleware_token_with_invalid_subject_should_require_login() {
        let claim = generate_invalid_subject_claim();
        let (token, session_secret) = generate_token_and_key(claim);

        let app = create_service(&session_secret).await;
        let cookie = create_cookie_for_token(&token).await;

        let req = test::TestRequest::default()
            .insert_header(ContentType::plaintext())
            .cookie(cookie)
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let claim = generate_unset_subject_claim();
        let (token, session_secret) = generate_token_and_key(claim);

        let app = create_service(&session_secret).await;
        let cookie = create_cookie_for_token(&token).await;

        let req = test::TestRequest::default()
            .insert_header(ContentType::plaintext())
            .cookie(cookie)
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn middleware_invalid_token_should_require_login() {
        let claim = generate_unset_subject_claim();
        let (_, session_secret) = generate_token_and_key(claim);
        let token = "someinvalidtestbytes".to_string();

        let app = create_service(&session_secret).await;
        let cookie = create_cookie_for_token(&token).await;

        let req = test::TestRequest::default()
            .insert_header(ContentType::plaintext())
            .cookie(cookie)
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    fn create_password_file(password: &str) -> tempfile::TempDir {
        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut OsRng);
        let hashed_password = argon2.hash_password(password.as_bytes(), &salt).unwrap();
        let config_path = tempfile::tempdir().unwrap();
        let file_path = config_path.path().join("password");
        let mut file = File::create(&file_path).unwrap();

        file.write_all(hashed_password.to_string().as_bytes())
            .unwrap();

        config_path
    }

    #[tokio::test]
    async fn middleware_correct_user_credentials_should_succeed_and_return_valid_token() {
        let session_secret = generate_hs256_key();

        let app = create_service(&String::from_utf8(session_secret.to_bytes()).unwrap()).await;

        let password = "some-password";
        let config_path = create_password_file(password);
        std::env::set_var("CONFIG_PATH", config_path.path());

        let encoded_password = BASE64_STANDARD.encode(format!(":{password}"));

        let req = test::TestRequest::default()
            .insert_header(ContentType::plaintext())
            .insert_header(("Authorization", format!("Basic {encoded_password}")))
            .to_request();
        println!("req: {req:#?}");
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
    }

    #[tokio::test]
    async fn middleware_invalid_user_credentials_should_return_unauthorized_error() {
        let session_secret = generate_hs256_key();

        let app = create_service(&String::from_utf8(session_secret.to_bytes()).unwrap()).await;

        let password = "some-password";
        let config_path = create_password_file(password);
        std::env::set_var("CONFIG_PATH", config_path.path());

        let encoded_password = BASE64_STANDARD.encode(":some-other-password");

        let req = test::TestRequest::default()
            .insert_header(ContentType::plaintext())
            .insert_header(("Authorization", format!("Basic {encoded_password}")))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn verify_correct_token_should_succeed() {
        let claim = generate_valid_claim();
        let (token, key) = generate_token_and_key(claim);

        assert!(verify_token(token.as_str(), &key));
    }

    #[tokio::test]
    async fn verify_expired_token_should_fail() {
        let claim = generate_expired_claim();
        let (token, key) = generate_token_and_key(claim);

        assert!(!verify_token(token.as_str(), &key));
    }

    #[tokio::test]
    async fn verify_token_with_invalid_subject_should_fail() {
        let claim = generate_unset_subject_claim();
        let (token, key) = generate_token_and_key(claim);

        assert!(!verify_token(token.as_str(), &key));

        let claim = generate_invalid_subject_claim();
        let (token, key) = generate_token_and_key(claim);

        assert!(!verify_token(token.as_str(), &key));
    }

    #[tokio::test]
    async fn verify_token_with_invalid_token_should_fail() {
        let claim = generate_invalid_subject_claim();
        let (_, key) = generate_token_and_key(claim);
        let token = "someinvalidtestbytes".to_string();

        assert!(!verify_token(token.as_str(), &key));
    }

    #[tokio::test]
    async fn verify_user_with_unset_password_should_fail() {
        let basic_auth = BasicAuth::from(Basic::new("some-user", None::<&str>));

        let expected = false;

        let result = verify_user(basic_auth);

        assert_eq!(expected, result);
    }
}
