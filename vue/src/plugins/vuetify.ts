/**
 * plugins/vuetify.ts
 *
 * Framework documentation: https://vuetifyjs.com`
 */

// Styles
import "@mdi/font/css/materialdesignicons.css"
import "vuetify/lib/styles/main.css"
import * as components from "vuetify/components"
import * as directives from "vuetify/directives"
import * as labsComponents from "vuetify/labs/components"

// Composables
import { createVuetify } from "vuetify"
import { componentAliases, theme, themeDefaults } from "../theme/theme.default"

// https://vuetifyjs.com/en/introduction/why-vuetify/#feature-guides
export default createVuetify({
	aliases: componentAliases,
	defaults: themeDefaults,
	theme: theme,
	directives,
	components: { ...components, ...labsComponents }
})
