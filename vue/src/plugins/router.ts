import { createRouter, createWebHistory } from "vue-router"
import { getUser, login } from "../auth/auth-service"
import Callback from "../pages/Callback.vue"
import DeviceOverview from "../pages/DeviceOverview.vue"
import DeviceUpdate from "../pages/DeviceUpdate.vue"
import Login from "../pages/Login.vue"
import SetPassword from "../pages/SetPassword.vue"
import UpdatePassword from "../pages/UpdatePassword.vue"

const routes = [
	{ path: "/", component: DeviceOverview, meta: { text: "Device", requiresAuth: true } },
	{ path: "/update", component: DeviceUpdate, meta: { text: "Update", requiresAuth: true } },
	{ path: "/login", component: Login },
	{ path: "/set-password", component: SetPassword, meta: { requiresPortalAuth: true } },
	{ path: "/update-password", component: UpdatePassword, meta: { requiresAuth: true } },
	{ path: "/auth-callback", component: Callback }
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
