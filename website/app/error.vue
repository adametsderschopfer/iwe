<script setup lang="ts">
import { catalogs } from '~/i18n'

const route = useRoute()
const messages = computed(() =>
  route.path.startsWith('/ru')
    ? catalogs.ru
    : route.path.startsWith('/es')
      ? catalogs.es
      : catalogs.en,
)
</script>

<template>
  <main class="error-page shell">
    <h1>{{ messages.error.title }}</h1>
    <p>{{ messages.error.description }}</p>
    <NuxtLink class="button" to="/">{{ messages.error.back }}</NuxtLink>
  </main>
</template>

<style scoped>
.error-page {
  padding-top: 20vh;
}
.error-page h1 {
  font-size: clamp(32px, 6vw, 64px);
  font-weight: 500;
  letter-spacing: -0.04em;
}
.error-page p {
  margin: 24px 0 32px;
  color: var(--muted);
}
</style>
