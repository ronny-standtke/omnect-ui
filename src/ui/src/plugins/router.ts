import { createRouter, createWebHistory } from "vue-router"
import { getUser, login } from "../auth/auth-service"
import Callback from "../pages/Callback.vue"
import DeviceOverview from "../pages/DeviceOverview.vue"
import DeviceUpdate from "../pages/DeviceUpdate.vue"
import Login from "../pages/Login.vue"
import Network from "../pages/Network.vue"
import SetPassword from "../pages/SetPassword.vue"
import UpdatePassword from "../pages/UpdatePassword.vue"

const routes = [
	{ path: "/", component: DeviceOverview, meta: { text: "Device", requiresAuth: true, showMenu: true } },
	{ path: "/network", component: Network, meta: { text: "Network", requiresAuth: true, showMenu: true } },
	{ path: "/update", component: DeviceUpdate, meta: { text: "Update", requiresAuth: true, showMenu: true } },
	{ path: "/login", component: Login, meta: { showMenu: false } },
	{ path: "/set-password", component: SetPassword, meta: { requiresPortalAuth: true, showMenu: false } },
	{ path: "/update-password", component: UpdatePassword, meta: { requiresAuth: true, showMenu: true } },
	{ path: "/auth-callback", component: Callback, meta: { showMenu: false } }
]

const router = createRouter({
	history: createWebHistory(),
	routes
})

router.beforeEach(async (to, _, next) => {
	if (to.meta.requiresPortalAuth) {
		const user = await getUser()
		if (!user || user.expired) {
			return login()
		}
	}
	if (to.meta.requiresAuth) {
		const res = await fetch("token/refresh")
		if (!res.ok) {
			next("/login")
		}
	}
	next()
})

export default router
