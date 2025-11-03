use actix_web::{App, http::header::ContentType, test, web};
use omnect_ui::api::Api;
use omnect_ui::keycloak_client::TokenClaims;
use std::path::PathBuf;

#[mockall_double::double]
use omnect_ui::{
    keycloak_client::SingleSignOnProvider, omnect_device_service_client::DeviceServiceClient,
};

async fn call_validate(
    api: Api<DeviceServiceClient, SingleSignOnProvider>,
) -> actix_web::dev::ServiceResponse {
    let app = test::init_service(App::new().app_data(web::Data::new(api)).route(
        "/validate",
        web::post().to(Api::<DeviceServiceClient, SingleSignOnProvider>::validate_portal_token),
    ))
    .await;
    let req = test::TestRequest::post()
        .uri("/validate")
        .insert_header(ContentType::plaintext())
        .set_payload("dummy")
        .to_request();
    test::call_service(&app, req).await
}

fn make_claims(role: &str, tenant: &str, fleets: Option<Vec<&str>>) -> TokenClaims {
    TokenClaims {
        roles: Some(vec![role.to_string()]),
        tenant_list: Some(vec![tenant.to_string()]),
        fleet_list: fleets.map(|fs| fs.into_iter().map(|f| f.to_string()).collect()),
    }
}

fn make_api(
    fleet_id: &'static str,
    claims: TokenClaims,
    tenant: &str,
) -> Api<DeviceServiceClient, SingleSignOnProvider> {
    let mut device_service_client_mock = DeviceServiceClient::default();
    device_service_client_mock
        .expect_fleet_id()
        .returning(|| Ok(fleet_id.to_string()));
    let mut single_sign_on_provider_mock = SingleSignOnProvider::default();
    single_sign_on_provider_mock
        .expect_verify_token()
        .returning(move |_| Ok(claims.clone()));

    Api {
        service_client: device_service_client_mock,
        single_sign_on: single_sign_on_provider_mock,
        index_html: PathBuf::from("/dev/null"),
        tenant: tenant.to_string(),
    }
}

async fn assert_status(
    api: Api<DeviceServiceClient, SingleSignOnProvider>,
    expected: actix_web::http::StatusCode,
) {
    let resp = call_validate(api).await;
    assert_eq!(resp.status(), expected);
}

#[tokio::test]
async fn validate_portal_token_fleet_admin_should_succeed() {
    let claims = make_claims("FleetAdministrator", "cp", None);
    let api = make_api("Fleet1", claims, "cp");
    assert_status(api, actix_web::http::StatusCode::OK).await;
}

#[tokio::test]
async fn validate_portal_token_fleet_admin_invalid_tenant_should_fail() {
    let claims = make_claims("FleetAdministrator", "invalid_tenant", None);
    let api = make_api("Fleet1", claims, "cp");
    assert_status(api, actix_web::http::StatusCode::UNAUTHORIZED).await;
}

#[tokio::test]
async fn validate_portal_token_fleet_operator_should_succeed() {
    let claims = make_claims("FleetOperator", "cp", Some(vec!["Fleet1", "Fleet2"]));
    let api = make_api("Fleet1", claims, "cp");
    assert_status(api, actix_web::http::StatusCode::OK).await;
}

#[tokio::test]
async fn validate_portal_token_fleet_operator_invalid_fleet_should_fail() {
    let claims = make_claims("FleetOperator", "cp", Some(vec!["Fleet2"]));
    let api = make_api("Fleet1", claims, "cp");
    assert_status(api, actix_web::http::StatusCode::UNAUTHORIZED).await;
}

#[tokio::test]
async fn validate_portal_token_fleet_observer_should_fail() {
    let claims = make_claims("FleetObserver", "cp", None);
    let api = make_api("Fleet1", claims, "cp");
    assert_status(api, actix_web::http::StatusCode::UNAUTHORIZED).await;
}
