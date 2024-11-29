use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, FromRequest, HttpMessage, HttpResponse,
};
use actix_web_httpauth::extractors::{basic::BasicAuth, bearer::BearerAuth};
use anyhow::{Context, Result};
use jwt_simple::prelude::*;
use log::error;
use std::{
    future::{ready, Future, Ready},
    pin::Pin,
    rc::Rc,
};

pub const TOKEN_EXPIRE_HOURS: u64 = 2;

pub struct BearerAuthMw;

impl<S, B> Transform<S, ServiceRequest> for BearerAuthMw
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = BearerAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(BearerAuthMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct BearerAuthMiddleware<S> {
    service: Rc<S>,
}

type LocalBoxFuture<T> = Pin<Box<dyn Future<Output = T> + 'static>>;

impl<S, B> Service<ServiceRequest> for BearerAuthMiddleware<S>
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
        let service = Rc::clone(&self.service);

        Box::pin(async move {
            let mut payload = req.take_payload().take();

            let auth = match BearerAuth::from_request(req.request(), &mut payload).await {
                Ok(b) => b,
                Err(_) => {
                    error!("no auth header");
                    return Ok(unauthorized_error(req).map_into_right_body());
                }
            };

            match verify_token(auth) {
                Ok(true) => {
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                }
                Ok(false) => Ok(unauthorized_error(req).map_into_right_body()),
                Err(e) => {
                    error!("user not authorized {}", e);
                    Ok(unauthorized_error(req).map_into_right_body())
                }
            }
        })
    }
}

pub fn verify_token(auth: BearerAuth) -> Result<bool> {
    let key = std::env::var("CENTRIFUGO_TOKEN_HMAC_SECRET_KEY").context("missing jwt secret")?;
    let key = HS256Key::from_bytes(key.as_bytes());
    let options = VerificationOptions {
        accept_future: true,
        time_tolerance: Some(Duration::from_mins(15)),
        max_validity: Some(Duration::from_hours(TOKEN_EXPIRE_HOURS)),
        required_subject: Some("omnect-ui".to_string()),
        ..Default::default()
    };

    Ok(key
        .verify_token::<NoCustomClaims>(auth.token(), Some(options))
        .is_ok())
}

pub struct BasicAuthMw;

impl<S, B> Transform<S, ServiceRequest> for BasicAuthMw
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = BasicAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(BasicAuthMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct BasicAuthMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for BasicAuthMiddleware<S>
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
        let service = Rc::clone(&self.service);

        Box::pin(async move {
            let mut payload = req.take_payload().take();

            let auth = match BasicAuth::from_request(req.request(), &mut payload).await {
                Ok(b) => b,
                Err(_) => {
                    error!("no auth header");
                    return Ok(unauthorized_error(req).map_into_right_body());
                }
            };

            match verify_user(auth) {
                Ok(true) => {
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                }
                Ok(false) => Ok(unauthorized_error(req).map_into_right_body()),
                Err(e) => {
                    error!("user not authorized {}", e);
                    Ok(unauthorized_error(req).map_into_right_body())
                }
            }
        })
    }
}

fn verify_user(auth: BasicAuth) -> Result<bool> {
    let user = std::env::var("LOGIN_USER").context("login_token: missing user")?;
    let password = std::env::var("LOGIN_PASSWORD").context("login_token: missing password")?;
    Ok(auth.user_id() == user && auth.password() == Some(&password))
}

fn unauthorized_error(req: ServiceRequest) -> ServiceResponse {
    let http_res = HttpResponse::Unauthorized().finish();
    let (http_req, _) = req.into_parts();
    ServiceResponse::new(http_req, http_res)
}
