import vue from "@vitejs/plugin-vue"
import fs from "node:fs"
import path from "node:path"
import UnoCSS from "unocss/vite"
import { defineConfig } from "vite"
import Vuetify, { transformAssetUrls } from "vite-plugin-vuetify"

// https://vite.dev/config/
export default defineConfig({
	plugins: [
		vue({
			template: { transformAssetUrls }
		}),
		Vuetify() as any,
		UnoCSS()
	],
	css: {
		preprocessorOptions: {
			sass: {
				api: "modern-compiler"
			}
		}
	},
	build: {
		chunkSizeWarningLimit: 1000,
		rollupOptions: {
			output: {
				manualChunks: {
					vue: ["vue", "vue-router", "@vueuse/core"],
					vuetify: ["vuetify"]
				}
			}
		}
	},
    preview: process.env.VITE_HTTPS === 'true' ? {
        port: 5173,
        https: {
            key: fs.readFileSync(path.resolve(__dirname, '../../temp/certs/server.key.pem')),
            cert: fs.readFileSync(path.resolve(__dirname, '../../temp/certs/server.cert.pem')),
        }
    } : {
        port: 5173
    }
})
