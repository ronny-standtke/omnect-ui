<template>
  <v-text-field
    :label="label"
    :append-inner-icon="visible ? 'mdi-eye-off' : 'mdi-eye'"
    :type="visible ? 'text' : 'password'"
    :model-value="modelValue"
    @update:model-value="$emit('update:modelValue', $event)"
    @click:append-inner="visible = !visible"
    :rules="rules"
    :disabled="disabled"
    variant="outlined"
    density="comfortable"
  />
</template>

<script setup lang="ts">
import { ref } from 'vue'

interface Props {
  label?: string
  modelValue: string
  rules?: Array<(value: string) => boolean | string>
  disabled?: boolean
}

withDefaults(defineProps<Props>(), {
  label: 'Password',
  disabled: false,
})

defineEmits<{
  'update:modelValue': [value: string]
}>()

const visible = ref(false)
</script>
