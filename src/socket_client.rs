use actix_web::http::StatusCode;
use anyhow::{Context, Result, ensure};
use http_body_util::BodyExt;
use hyper::{Request, Uri as HyperUri};
use hyper_util::client::legacy::Client;
use hyperlocal::{UnixClientExt, UnixConnector};
use log::info;
use serde::Serialize;
use std::sync::OnceLock;

static SOCKET_CLIENT: OnceLock<SocketClient> = OnceLock::new();

#[derive(Clone)]
pub struct SocketClient {
    client: Client<UnixConnector, String>,
}

impl Default for SocketClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SocketClient {
    pub fn new() -> Self {
        SOCKET_CLIENT
            .get_or_init(|| {
                let client = Client::unix();
                SocketClient { client }
            })
            .clone()
    }

    pub async fn post_with_json_body(
        &self,
        uri: &HyperUri,
        body: impl Serialize,
    ) -> Result<String> {
        let request = Request::builder()
            .uri(uri)
            .method("POST")
            .header("Host", "localhost")
            .body(serde_json::to_string(&body).unwrap_or_default())
            .context(format!("post_with_json_body: build request {uri}"))?;

        self.send_request(request).await
    }

    pub async fn post_with_empty_body(&self, uri: &HyperUri) -> Result<String> {
        let request = Request::builder()
            .uri(uri)
            .method("POST")
            .header("Host", "localhost")
            .body(String::new())
            .context(format!("post_with_json_body: build request {uri}"))?;

        self.send_request(request).await
    }

    pub async fn get_with_empty_body(&self, uri: &HyperUri) -> Result<String> {
        let request = Request::builder()
            .uri(uri)
            .method("GET")
            .header("Host", "localhost")
            .body(String::new())
            .context(format!("get_with_empty_body: build request {uri}"))?;

        self.send_request(request).await
    }

    pub async fn delete_with_empty_body(&self, uri: &HyperUri) -> Result<String> {
        let request = Request::builder()
            .uri(uri)
            .method("DELETE")
            .header("Host", "localhost")
            .body(String::new())
            .context(format!("delete_with_empty_body: build request {uri}"))?;

        self.send_request(request).await
    }

    async fn send_request(&self, request: Request<String>) -> Result<String> {
        info!("send request: {request:?}");

        let res = self
            .client
            .request(request.clone())
            .await
            .context("send request failed")?;

        let status_code =
            StatusCode::from_u16(res.status().as_u16()).context("get status code failed")?;

        let body = res
            .collect()
            .await
            .context("collect response body failed")?;

        let body =
            String::from_utf8(body.to_bytes().to_vec()).context("get response body failed")?;

        ensure!(
            status_code.is_success(),
            "request: {request:?} failed with status code: {status_code} and body: {body}"
        );

        Ok(body)
    }
}
