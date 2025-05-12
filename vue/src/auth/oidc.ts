import { InMemoryWebStorage, UserManager, WebStorageStateStore } from "oidc-client-ts"

const config = window.__APP_CONFIG__

const oidcConfig = {
	authority: config.KEYCLOAK_URL,
	client_id: "omnect-ui",
	redirect_uri: `https://${window.location.hostname}:${window.location.port}/auth-callback`,
	response_type: "code",
	scope: "openid profile email",
	post_logout_redirect_uri: `https://${window.location.hostname}:${window.location.port}/`,
	userStore: new WebStorageStateStore({ store: new InMemoryWebStorage() })
}

const userManager = new UserManager(oidcConfig)

export default userManager
