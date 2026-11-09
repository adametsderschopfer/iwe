export default defineNuxtConfig({
  compatibilityDate: '2026-11-09',
  devtools: { enabled: false },
  modules: ['@nuxt/eslint'],
  css: ['~/assets/main.css'],
  app: {
    head: {
      meta: [{ name: 'theme-color', content: '#121311' }],
    },
  },
  nitro: {
    prerender: {
      routes: ['/', '/ru/', '/es/'],
      failOnError: true,
    },
  },
  typescript: { strict: true },
})
