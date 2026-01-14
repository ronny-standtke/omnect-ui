//! Authorization service
//!
//! Handles token validation and role-based access control independent of HTTP concerns.

use crate::{
    config::AppConfig, keycloak_client::SingleSignOnProvider,
    omnect_device_service_client::DeviceServiceClient,
};
use anyhow::{Result, bail, ensure};

/// Service for authorization operations
pub struct AuthorizationService;

impl AuthorizationService {
    /// Validate SSO token and check user claims for authorization
    ///
    /// Uses the tenant configuration from AppConfig.
    ///
    /// # Arguments
    /// * `single_sign_on` - Single sign-on provider for token verification
    /// * `service_client` - Device service client for fleet ID lookup
    /// * `token` - The authentication token to validate
    ///
    /// # Returns
    /// Result indicating success or authorization failure
    ///
    /// # Authorization Rules
    /// - User must have tenant in their tenant_list
    /// - FleetAdministrator role grants full access
    /// - FleetOperator role requires fleet_id in fleet_list
    pub async fn validate_token_and_claims<ServiceClient, SingleSignOn>(
        single_sign_on: &SingleSignOn,
        service_client: &ServiceClient,
        token: &str,
    ) -> Result<()>
    where
        ServiceClient: DeviceServiceClient,
        SingleSignOn: SingleSignOnProvider,
    {
        let claims = single_sign_on.verify_token(token).await?;
        let tenant = &AppConfig::get().tenant;

        // Validate tenant authorization
        let Some(tenant_list) = &claims.tenant_list else {
            bail!("failed to authorize user: no tenant list in token");
        };
        ensure!(
            tenant_list.contains(tenant),
            "failed to authorize user: insufficient permissions for tenant"
        );

        // Validate role-based authorization
        let Some(roles) = &claims.roles else {
            bail!("failed to authorize user: no roles in token");
        };

        // FleetAdministrator has full access
        if roles.iter().any(|r| r == "FleetAdministrator") {
            return Ok(());
        }

        // FleetOperator requires fleet validation
        if roles.iter().any(|r| r == "FleetOperator") {
            let Some(fleet_list) = &claims.fleet_list else {
                bail!("failed to authorize user: no fleet list in token");
            };
            let fleet_id = service_client.fleet_id().await?;
            ensure!(
                fleet_list.contains(&fleet_id),
                "failed to authorize user: insufficient permissions for fleet"
            );
            return Ok(());
        }

        bail!("failed to authorize user: insufficient role permissions")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keycloak_client::TokenClaims;

    #[cfg(feature = "mock")]
    use mockall_double::double;

    #[cfg(feature = "mock")]
    #[double]
    use crate::keycloak_client::SingleSignOnProvider;

    #[cfg(feature = "mock")]
    #[double]
    use crate::omnect_device_service_client::DeviceServiceClient;

    fn create_claims(
        roles: Option<Vec<&str>>,
        tenant_list: Option<Vec<&str>>,
        fleet_list: Option<Vec<&str>>,
    ) -> TokenClaims {
        TokenClaims {
            roles: roles.map(|r| r.into_iter().map(String::from).collect()),
            tenant_list: tenant_list.map(|t| t.into_iter().map(String::from).collect()),
            fleet_list: fleet_list.map(|f| f.into_iter().map(String::from).collect()),
        }
    }

    mod fleet_administrator {
        use super::*;

        #[tokio::test]
        async fn with_valid_tenant_succeeds() {
            let mut sso_mock = SingleSignOnProvider::default();
            sso_mock.expect_verify_token().returning(|_| {
                Box::pin(async {
                    Ok(create_claims(
                        Some(vec!["FleetAdministrator"]),
                        Some(vec!["cp"]),
                        None,
                    ))
                })
            });

            let device_mock = DeviceServiceClient::default();

            let result = AuthorizationService::validate_token_and_claims(
                &sso_mock,
                &device_mock,
                "valid_token",
            )
            .await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn with_invalid_tenant_fails() {
            let mut sso_mock = SingleSignOnProvider::default();
            sso_mock.expect_verify_token().returning(|_| {
                Box::pin(async {
                    Ok(create_claims(
                        Some(vec!["FleetAdministrator"]),
                        Some(vec!["invalid_tenant"]),
                        None,
                    ))
                })
            });

            let device_mock = DeviceServiceClient::default();

            let result = AuthorizationService::validate_token_and_claims(
                &sso_mock,
                &device_mock,
                "valid_token",
            )
            .await;

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("insufficient permissions for tenant")
            );
        }

        #[tokio::test]
        async fn with_multiple_tenants_including_valid_succeeds() {
            let mut sso_mock = SingleSignOnProvider::default();
            sso_mock.expect_verify_token().returning(|_| {
                Box::pin(async {
                    Ok(create_claims(
                        Some(vec!["FleetAdministrator"]),
                        Some(vec!["other_tenant", "cp", "another_tenant"]),
                        None,
                    ))
                })
            });

            let device_mock = DeviceServiceClient::default();

            let result = AuthorizationService::validate_token_and_claims(
                &sso_mock,
                &device_mock,
                "valid_token",
            )
            .await;

            assert!(result.is_ok());
        }
    }

    mod fleet_operator {
        use super::*;

        #[tokio::test]
        async fn with_matching_fleet_succeeds() {
            let mut sso_mock = SingleSignOnProvider::default();
            sso_mock.expect_verify_token().returning(|_| {
                Box::pin(async {
                    Ok(create_claims(
                        Some(vec!["FleetOperator"]),
                        Some(vec!["cp"]),
                        Some(vec!["fleet-123"]),
                    ))
                })
            });

            let mut device_mock = DeviceServiceClient::default();
            device_mock
                .expect_fleet_id()
                .returning(|| Box::pin(async { Ok("fleet-123".to_string()) }));

            let result = AuthorizationService::validate_token_and_claims(
                &sso_mock,
                &device_mock,
                "valid_token",
            )
            .await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn with_non_matching_fleet_fails() {
            let mut sso_mock = SingleSignOnProvider::default();
            sso_mock.expect_verify_token().returning(|_| {
                Box::pin(async {
                    Ok(create_claims(
                        Some(vec!["FleetOperator"]),
                        Some(vec!["cp"]),
                        Some(vec!["fleet-456"]),
                    ))
                })
            });

            let mut device_mock = DeviceServiceClient::default();
            device_mock
                .expect_fleet_id()
                .returning(|| Box::pin(async { Ok("fleet-123".to_string()) }));

            let result = AuthorizationService::validate_token_and_claims(
                &sso_mock,
                &device_mock,
                "valid_token",
            )
            .await;

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("insufficient permissions for fleet")
            );
        }

        #[tokio::test]
        async fn with_multiple_fleets_including_match_succeeds() {
            let mut sso_mock = SingleSignOnProvider::default();
            sso_mock.expect_verify_token().returning(|_| {
                Box::pin(async {
                    Ok(create_claims(
                        Some(vec!["FleetOperator"]),
                        Some(vec!["cp"]),
                        Some(vec!["fleet-456", "fleet-123", "fleet-789"]),
                    ))
                })
            });

            let mut device_mock = DeviceServiceClient::default();
            device_mock
                .expect_fleet_id()
                .returning(|| Box::pin(async { Ok("fleet-123".to_string()) }));

            let result = AuthorizationService::validate_token_and_claims(
                &sso_mock,
                &device_mock,
                "valid_token",
            )
            .await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn without_fleet_list_fails() {
            let mut sso_mock = SingleSignOnProvider::default();
            sso_mock.expect_verify_token().returning(|_| {
                Box::pin(async {
                    Ok(create_claims(
                        Some(vec!["FleetOperator"]),
                        Some(vec!["cp"]),
                        None,
                    ))
                })
            });

            let device_mock = DeviceServiceClient::default();

            let result = AuthorizationService::validate_token_and_claims(
                &sso_mock,
                &device_mock,
                "valid_token",
            )
            .await;

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("no fleet list in token")
            );
        }

        #[tokio::test]
        async fn with_invalid_tenant_fails() {
            let mut sso_mock = SingleSignOnProvider::default();
            sso_mock.expect_verify_token().returning(|_| {
                Box::pin(async {
                    Ok(create_claims(
                        Some(vec!["FleetOperator"]),
                        Some(vec!["invalid_tenant"]),
                        Some(vec!["fleet-123"]),
                    ))
                })
            });

            let device_mock = DeviceServiceClient::default();

            let result = AuthorizationService::validate_token_and_claims(
                &sso_mock,
                &device_mock,
                "valid_token",
            )
            .await;

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("insufficient permissions for tenant")
            );
        }
    }

    mod fleet_observer {
        use super::*;

        #[tokio::test]
        async fn with_valid_tenant_fails() {
            let mut sso_mock = SingleSignOnProvider::default();
            sso_mock.expect_verify_token().returning(|_| {
                Box::pin(async {
                    Ok(create_claims(
                        Some(vec!["FleetObserver"]),
                        Some(vec!["cp"]),
                        None,
                    ))
                })
            });

            let device_mock = DeviceServiceClient::default();

            let result = AuthorizationService::validate_token_and_claims(
                &sso_mock,
                &device_mock,
                "valid_token",
            )
            .await;

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("insufficient role permissions")
            );
        }
    }

    mod missing_claims {
        use super::*;

        #[tokio::test]
        async fn without_tenant_list_fails() {
            let mut sso_mock = SingleSignOnProvider::default();
            sso_mock.expect_verify_token().returning(|_| {
                Box::pin(async { Ok(create_claims(Some(vec!["FleetAdministrator"]), None, None)) })
            });

            let device_mock = DeviceServiceClient::default();

            let result = AuthorizationService::validate_token_and_claims(
                &sso_mock,
                &device_mock,
                "valid_token",
            )
            .await;

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("no tenant list in token")
            );
        }

        #[tokio::test]
        async fn without_roles_fails() {
            let mut sso_mock = SingleSignOnProvider::default();
            sso_mock
                .expect_verify_token()
                .returning(|_| Box::pin(async { Ok(create_claims(None, Some(vec!["cp"]), None)) }));

            let device_mock = DeviceServiceClient::default();

            let result = AuthorizationService::validate_token_and_claims(
                &sso_mock,
                &device_mock,
                "valid_token",
            )
            .await;

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("no roles in token")
            );
        }
    }

    mod token_verification {
        use super::*;

        #[tokio::test]
        async fn with_invalid_token_fails() {
            let mut sso_mock = SingleSignOnProvider::default();
            sso_mock
                .expect_verify_token()
                .returning(|_| Box::pin(async { Err(anyhow::anyhow!("invalid token signature")) }));

            let device_mock = DeviceServiceClient::default();

            let result = AuthorizationService::validate_token_and_claims(
                &sso_mock,
                &device_mock,
                "invalid_token",
            )
            .await;

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("invalid token signature")
            );
        }
    }
}
