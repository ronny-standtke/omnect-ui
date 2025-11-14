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
