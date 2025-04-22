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

use crate::{api::Api, validate_password};

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
        let service = Rc::clone(&self.service);

        Box::pin(async move {
            let Some(api_config) = req.app_data::<Data<Api>>().cloned() else {
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
                    .is_ok_and(|res| res)
            {
                let res = service.call(req).await?;
                Ok(res.map_into_left_body())
            } else {
                let mut payload = req.take_payload().take();

                let Ok(auth) = BasicAuth::from_request(req.request(), &mut payload).await else {
                    return Ok(unauthorized_error(req).map_into_right_body());
                };

                if verify_user(auth).await {
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                } else {
                    Ok(unauthorized_error(req).map_into_right_body())
                }
            }
        })
    }
}

pub fn verify_token(token: &str, centrifugo_client_token_hmac_secret_key: &str) -> Result<bool> {
    let key = HS256Key::from_bytes(centrifugo_client_token_hmac_secret_key.as_bytes());
    let options = VerificationOptions {
        accept_future: true,
        time_tolerance: Some(Duration::from_mins(15)),
        max_validity: Some(Duration::from_hours(TOKEN_EXPIRE_HOURS)),
        required_subject: Some("omnect-ui".to_string()),
        ..Default::default()
    };

    Ok(key
        .verify_token::<NoCustomClaims>(token, Some(options))
        .is_ok())
}

async fn verify_user(auth: BasicAuth) -> bool {
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
