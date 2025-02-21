import type { ThemeDefinition } from "vuetify"
import { VBtn } from "vuetify/components/VBtn"
import { VSelect } from "vuetify/components/VSelect"

const omnectTheme: ThemeDefinition = {
	dark: false,
	colors: {
		background: "#f4f5f7",
		surface: "#fff",
		primary: "#677680",
		"on-primary": "#fff",
		"primary-darken-1": "#3700B3",
		secondary: "#0094b1",
		"secondary-darken-1": "#018786",
		error: "#b3101d",
		critical: "#b3101d",
		info: "#2196F3",
		success: "#6ca425",
		warning: "#ffb100",
		btn: "#000"
	}
}

export const themeDefaults = {
	VIconBtn: {
		color: "btn",
		variant: "outlined",
		rounded: true,
		style: [{ borderRadius: "4px" }, { backgroundColor: "#fff" }]
	},
	VTextField: {
		color: "primary",
		variant: "outlined",
		density: "compact"
	},
	VField: {
		style: [{ backgroundColor: "#fff" }]
	},
	VSelect: {
		density: "compact",
		style: [{ backgroundColor: "#fff" }]
	},
	VSelectFilter: {
		variant: "outlined",
		density: "compact"
		//style: [{ backgroundColor: "#" }]
	},
	VFilledButton: {
		variant: "elevated",
		rounded: "md",
		style: [{ color: "#fff" }, { backgroundColor: "#677680" }]
	},
	VOutlinedButton: {
		variant: "outlined",
		rounded: "md"
	},
	VDangerButton: {
		variant: "elevated",
		rounded: "md",
		style: [{ color: "#fff" }, { backgroundColor: omnectTheme.colors?.error }]
	},
	VRadioGroup: {
		color: "secondary"
	},
	VTextarea: {
		variant: "outlined"
	},
	VCheckboxBtn: {
		color: "secondary"
	},
	VDataTableServer: {
		itemsPerPageOptions: [
			{ value: 10, title: "10" },
			{ value: 25, title: "25" },
			{ value: 50, title: "50" },
			{ value: 100, title: "100" }
		]
	},
	VTable: {
		style: [{ fontSize: "1rem" }]
	}
}

export const componentAliases = {
	VDangerButton: VBtn,
	VFilledButton: VBtn,
	VOutlinedButton: VBtn,
	VIconBtn: VBtn,
	VSelectFilter: VSelect
}

export const theme = {
	defaultTheme: "omnectTheme",
	themes: {
		omnectTheme
	}
}
