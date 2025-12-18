import { createRouter, createWebHistory } from "vue-router"
import { getUser, login } from "../auth/auth-service"
import { useCore } from "../composables/useCore"
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
	{ path: "/login", component: Login, meta: { showMenu: false, guestOnly: true } },
	{ path: "/set-password", component: SetPassword, meta: { requiresPortalAuth: true, showMenu: false } },
	{ path: "/update-password", component: UpdatePassword, meta: { requiresAuth: true, showMenu: true } },
	{ path: "/auth-callback", component: Callback, meta: { showMenu: false } }
]

const router = createRouter({
	history: createWebHistory(),
	routes
})

router.beforeEach(async (to, _, next) => {
	const { viewModel } = useCore()

	if (to.meta.guestOnly && viewModel.is_authenticated) {
		next("/")
		return
	}

	if (to.meta.requiresPortalAuth) {
		const user = await getUser()
		if (!user || user.expired) {
			return login()
		}
	}
	if (to.meta.requiresAuth) {
		// Rely on the Core's authentication state as the single source of truth
		if (!viewModel.is_authenticated) {
			next("/login")
			return
		}
	}
	next()
})

export default router
