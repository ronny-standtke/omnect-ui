import userManager from "./oidc"

export const login = () => userManager.signinRedirect()
export const logout = () => userManager.signoutRedirect()
export const getUser = () => userManager.getUser()
export const handleRedirectCallback = () => userManager.signinRedirectCallback()
