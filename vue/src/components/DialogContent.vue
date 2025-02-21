<script setup lang="ts" >
import { computed } from "vue"

interface DialogProps {
	title: string
	dialogType?: "default" | "Warning" | "Error"
	showClose?: boolean
}

const props = withDefaults(defineProps<DialogProps>(), {
	dialogType: "default",
	showClose: true
})

const emits = defineEmits<(e: "close") => void>()
const dialogType = computed(() => props.dialogType ?? "default")
const titleColor = computed(() => {
	switch (dialogType.value) {
		case "Warning":
			return "warning"
		case "Error":
			return "error"
		default:
			return "primary"
	}
})
</script>

<template>
  <v-card>
    <v-toolbar dark :color="titleColor">
      <v-toolbar-title>
        <div>{{ props.title }}</div>
      </v-toolbar-title>
      <v-btn v-if="showClose" icon dark @click="emits('close')">
        <v-icon>mdi-close</v-icon>
      </v-btn>
    </v-toolbar>
    <div class="p-4">
      <slot></slot>
    </div>
  </v-card>
</template>