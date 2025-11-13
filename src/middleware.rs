use crate::auth::{TokenManager, validate_password};
use actix_session::SessionExt;
use actix_web::{
    Error, FromRequest, HttpMessage, HttpResponse,
    body::EitherBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    web,
};
use actix_web_httpauth::extractors::basic::BasicAuth;
use anyhow::Result;
use log::error;
use std::{
    future::{Future, Ready, ready},
    pin::Pin,
    rc::Rc,
};

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
            let token = match req.get_session().get::<String>("token") {
                Ok(token) => token.unwrap_or_default(),
                Err(e) => {
                    error!("failed to get session. {e:#}");
                    String::new()
                }
            };

            // Extract TokenManager from app data
            let Some(token_manager) = req.app_data::<web::Data<TokenManager>>() else {
                error!("failed to get TokenManager.");
                return Ok(unauthorized_error(req).map_into_right_body());
            };

            if token_manager.verify_token(&token) {
                let res = service.call(req).await?;
                return Ok(res.map_into_left_body());
            }

            let mut payload = req.take_payload().take();

            let Ok(auth) = BasicAuth::from_request(req.request(), &mut payload).await else {
                return Ok(unauthorized_error(req).map_into_right_body());
            };

            let true = verify_user(auth) else {
                return Ok(unauthorized_error(req).map_into_right_body());
            };

            let res = service.call(req).await?;

            Ok(res.map_into_left_body())
        })
    }
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
pub mod tests {
    use super::*;
    use crate::config::AppConfig;

    const TOKEN_SUBJECT: &str = "omnect-ui";
    const TOKEN_EXPIRE_HOURS: u64 = 2;
    use actix_http::StatusCode;
    use actix_session::{
        SessionMiddleware,
        config::{BrowserSession, CookieContentSecurity},
        storage::{CookieSessionStore, SessionStore},
    };
    use actix_web::{
        App, HttpResponse, Responder,
        cookie::{Cookie, CookieJar, Key, SameSite},
        dev::ServiceResponse,
        http::header::ContentType,
        test, web,
    };
    use actix_web_httpauth::headers::authorization::Basic;
    use argon2::{
        Argon2,
        password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
    };
    use base64::prelude::*;
    use jwt_simple::claims::{JWTClaims, NoCustomClaims};
    use jwt_simple::prelude::*;
    use std::{collections::HashMap, fs::File, io::Write};

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
            subject: Some(TOKEN_SUBJECT.to_string()),
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
            subject: Some(TOKEN_SUBJECT.to_string()),
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

    fn generate_token(claim: JWTClaims<NoCustomClaims>) -> String {
        let key = HS256Key::from_bytes(AppConfig::get().centrifugo.client_token.as_bytes());
        key.authenticate(claim).unwrap()
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

    async fn create_service() -> impl actix_service::Service<
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

        let token_manager = TokenManager::new(AppConfig::get().centrifugo.client_token.as_str());

        test::init_service(
            App::new()
                .app_data(web::Data::new(token_manager))
                .wrap(session_middleware)
                .route("/", web::get().to(index).wrap(AuthMw)),
        )
        .await
    }

    async fn create_cookie_for_token(token: &str) -> Cookie<'_> {
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
        let token = generate_token(claim);

        let app = create_service().await;
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
        let token = generate_token(claim);

        let app = create_service().await;
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
        let token = generate_token(claim);

        let app = create_service().await;
        let cookie = create_cookie_for_token(&token).await;

        let req = test::TestRequest::default()
            .insert_header(ContentType::plaintext())
            .cookie(cookie)
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let claim = generate_unset_subject_claim();
        let token = generate_token(claim);

        let app = create_service().await;
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
        let _ = generate_token(claim);
        let token = "someinvalidtestbytes".to_string();

        let app = create_service().await;
        let cookie = create_cookie_for_token(&token).await;

        let req = test::TestRequest::default()
            .insert_header(ContentType::plaintext())
            .cookie(cookie)
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    fn setup_password_file(password: &str) {
        use crate::config::AppConfig;

        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut OsRng);
        let hashed_password = argon2.hash_password(password.as_bytes(), &salt).unwrap();

        let password_file = &AppConfig::get().paths.password_file;
        let config_dir = password_file.parent().unwrap();
        std::fs::create_dir_all(config_dir).unwrap();
        let mut file = File::create(password_file).unwrap();

        file.write_all(hashed_password.to_string().as_bytes())
            .unwrap();
    }

    #[tokio::test]
    async fn middleware_correct_user_credentials_should_succeed_and_return_valid_token() {
        let password = "some-password";
        setup_password_file(password);

        let app = create_service().await;

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
        let password = "some-password";
        setup_password_file(password);

        let app = create_service().await;

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
        let token = generate_token(claim);
        let token_manager = TokenManager::new(AppConfig::get().centrifugo.client_token.as_str());

        assert!(token_manager.verify_token(token.as_str()));
    }

    #[tokio::test]
    async fn verify_expired_token_should_fail() {
        let claim = generate_expired_claim();
        let token = generate_token(claim);
        let token_manager = TokenManager::new(AppConfig::get().centrifugo.client_token.as_str());

        assert!(!token_manager.verify_token(token.as_str()));
    }

    #[tokio::test]
    async fn verify_token_with_invalid_subject_should_fail() {
        let claim = generate_unset_subject_claim();
        let token = generate_token(claim);
        let token_manager = TokenManager::new(AppConfig::get().centrifugo.client_token.as_str());

        assert!(!token_manager.verify_token(token.as_str()));

        let claim = generate_invalid_subject_claim();
        let token = generate_token(claim);

        assert!(!token_manager.verify_token(token.as_str()));
    }

    #[tokio::test]
    async fn verify_token_with_invalid_token_should_fail() {
        let claim = generate_invalid_subject_claim();
        let _ = generate_token(claim);
        let token = "someinvalidtestbytes".to_string();
        let token_manager = TokenManager::new(AppConfig::get().centrifugo.client_token.as_str());

        assert!(!token_manager.verify_token(token.as_str()));
    }

    #[tokio::test]
    async fn verify_user_with_unset_password_should_fail() {
        let basic_auth = BasicAuth::from(Basic::new("some-user", None::<&str>));

        let expected = false;

        let result = verify_user(basic_auth);

        assert_eq!(expected, result);
    }
}
