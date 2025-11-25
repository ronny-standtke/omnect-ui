import { defineConfig, presetIcons, presetTypography, presetUno, transformerDirectives, transformerVariantGroup } from "unocss"

export default defineConfig({
	shortcuts: [
		["widget", "bg-white p-4 border-light-30 border-1 rounded"],
		["interactive-widget", "widget cursor-pointer hover:border-secondary shadow hover:shadow-md duration-300"],
		["section-title", "uppercase text-text-primary mb-2"],
		["button", "bg-primary hover:bg-gray-900 text-white rounded text-center h-8 py-1 px-4 capitalize cursor-pointer flex items-center"],
		["redButton", "bg-red-700 hover:bg-red-900 text-white rounded text-center h-8 py-1 px-4 capitalize cursor-pointer flex items-center"],
		[
			"blueButton",
			"bg-blue-700 hover:bg-blue-900 text-white rounded text-center h-8 py-1 px-4 capitalize cursor-pointer flex items-center"
		],
		["fleet_label", "bg-secondary p-2 text-sm rounded-md text-white"],
		["wizard-step", "flex flex-col h-60 w-40 overflow-hidden shadow-md w-[23rem] h-[27.5rem] select-none rounded-lg"],
		["wizard-step-header", "flex h-12 items-center px-5 gap-3 text-lg"],
		["wizard-step-content", "flex-1 flex flex-col text-sm bg-white"],
		["wizard-step-footer", "flex w-full bg-white"],
		["wizard-step-indicator", "flex flex-col items-center justify-center"],
		["wizard-step-text", "text-primary text-sm"],
		[
			"wizard-step-icon",
			"flex items-center justify-center relative rounded-full w-10 h-10 border-white border-2 transition-all duration-500"
		]
	],
	presets: [
		presetUno(),
		presetIcons({
			scale: 1.0,
			warn: true
		}),
		presetTypography()
	],
	theme: {
		colors: {
			secondary: "var(--color-secondary)",
			light: {
				5: "var(--color-grey-5)",
				10: "var(--color-grey-10)",
				30: "var(--color-grey-30)"
			},
			header: "var(--color-header)",
			background: "var(--color-background)",
			primary: "var(--color-primary)",
			signal: "var(--color-notification-setup-fill)",
			signalblue: "var(--color-notification-general-fill)",
			white_70: "var(--color-white-dimmed)",
			notifications: {
				setupfill: "var(--color-notification-setup-fill)",
				setuptext: "var(--color-notification-setup-text)",
				updatefill: "var(--color-notification-update-fill)",
				updatetext: "var(--color-notification-update-text)",
				generalfill: "var(--color-notification-general-fill)",
				commonfill: "var(--color-notification-common-fill)",
				generaltext: "var(--color-notification-general-text)",
				successfill: "var(--color-notification-success-fill)",
				cancelledfill: "var(--color-notification-cancelled-fill)",
				pendingfill: "var(--color-notification-pending-fill)"
			},
			text: {
				primary: "var(--color-text-primary)",
				secondary: "var(--color-text-secondary)"
			},
			fail: "var(--color-fail)"
		},
		boxShadow: {
			DEFAULT: "0 2px 5px rgba(0, 0, 0, 0.1)",
			md: "0 4px 5px rgba(0,0,0,0.2)"
		},
		screens: {
			"3xl": "1920px"
		},
		fontFamily: {
			sans: ['"Lato"']
		},
		fontSize: {
			xxs: ".6rem"
		},
		lineHeight: {
			"2.5": "0.6875rem"
		},
		gridTemplateColumns: {
			fleetnotification: "32px_1fr_32px"
		}
	},
	transformers: [transformerDirectives(), transformerVariantGroup()],
	safelist: "prose prose-sm m-auto text-left text-white_70".split(" ")
})
