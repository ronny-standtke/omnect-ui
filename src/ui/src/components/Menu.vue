<template>
  <v-menu close-delay="100" location="bottom end" open-delay="60" open-on-hover>
    <template #activator="{ props }">
      <slot name="activator" v-bind="{ props }" />
    </template>

    <v-sheet class="overflow-hidden" border rounded>
      <slot />
    </v-sheet>
  </v-menu>
</template>