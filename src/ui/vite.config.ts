import vue from "@vitejs/plugin-vue"
import UnoCSS from "unocss/vite"
import { defineConfig } from "vite"
import Vuetify, { transformAssetUrls } from "vite-plugin-vuetify"

// https://vite.dev/config/
export default defineConfig({
	plugins: [
		vue({
			template: { transformAssetUrls }
		}),
		Vuetify(),
		UnoCSS()
	],
	css: {
		preprocessorOptions: {
			sass: {
				api: "modern-compiler"
			}
		}
	}
})
