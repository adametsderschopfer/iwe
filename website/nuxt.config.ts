import releaseNotes from './app/release-notes/releases.json'

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
      routes: [
        '/',
        '/ru/',
        '/es/',
        ...['', '/ru', '/es'].flatMap((locale) => [
          `${locale}/releases/`,
          `${locale}/license/`,
          ...releaseNotes
            .filter((release) => release.items.length > 0)
            .map((release) => `${locale}/releases/${release.version}/`),
        ]),
      ],
      failOnError: true,
    },
  },
  typescript: { strict: true },
})
