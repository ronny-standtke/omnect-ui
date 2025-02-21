import { createRouter, createWebHistory } from "vue-router"

import DeviceOverview from "../pages/DeviceOverview.vue"
import Login from "../pages/Login.vue"

const routes = [
	{ path: "/", component: DeviceOverview },
	{ path: "/login", component: Login }
]

const router = createRouter({
	history: createWebHistory(),
	routes
})

export default router
