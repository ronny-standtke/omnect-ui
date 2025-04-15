import { createRouter, createWebHistory } from "vue-router"

import DeviceOverview from "../pages/DeviceOverview.vue"
import DeviceUpdate from "../pages/DeviceUpdate.vue"
import Login from "../pages/Login.vue"
import SetPassword from "../pages/SetPassword.vue"
import UpdatePassword from "../pages/UpdatePassword.vue"

const routes = [
	{ path: "/", component: DeviceOverview, meta: { text: "Device" } },
	{ path: "/update", component: DeviceUpdate, meta: { text: "Update" } },
	{ path: "/login", component: Login },
	{ path: "/set-password", component: SetPassword },
	{ path: "/update-password", component: UpdatePassword }
]

const router = createRouter({
	history: createWebHistory(),
	routes
})

export default router
