import process from 'node:process'
import releaseNotes from './app/release-notes/releases.json'

const releaseResponse = await fetch(
  'https://api.github.com/repos/adametsderschopfer/iwe-code/releases/latest',
  {
    headers: {
      Accept: 'application/vnd.github+json',
      ...(process.env.GH_TOKEN ? { Authorization: `Bearer ${process.env.GH_TOKEN}` } : {}),
    },
    signal: AbortSignal.timeout(30_000),
  },
)
if (!releaseResponse.ok) {
  throw new Error(`Cannot load the latest GitHub release: HTTP ${releaseResponse.status}`)
}
const release = (await releaseResponse.json()) as {
  tag_name: string
  draft: boolean
  prerelease: boolean
  html_url: string
  assets: { name: string; browser_download_url: string; size: number }[]
}
if (release.draft || release.prerelease || !/^v[0-9]+\.[0-9]+\.[0-9]+$/.test(release.tag_name)) {
  throw new Error('The latest GitHub release must be a published stable version')
}
const version = release.tag_name.slice(1)
const downloads: Record<string, string> = {}
for (const suffix of [
  'macos-arm64.dmg',
  'macos-x64.dmg',
  'windows-x64-setup.exe',
  'windows-x86-setup.exe',
  'linux-x64.tar.gz',
  'linux-x86.tar.gz',
]) {
  const asset = release.assets.find((item) => item.name === `iwe-code-${version}-${suffix}`)
  if (!asset || asset.size <= 0) {
    throw new Error(`The latest release is missing the download package: ${suffix}`)
  }
  downloads[suffix] = asset.browser_download_url
}

export default defineNuxtConfig({
  runtimeConfig: {
    public: {
      downloadRelease: {
        version,
        url: release.html_url,
        hasNotes: releaseNotes.some((record) => record.version === version),
        downloads,
      },
    },
  },
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
          `${locale}/modules/`,
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
