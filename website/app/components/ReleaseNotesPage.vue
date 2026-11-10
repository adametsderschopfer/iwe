<script setup lang="ts">
import { catalogs, languages, repository, type Locale } from '~/i18n'
import { categories, releaseNotes, releaseNotesPath } from '~/release-notes'

const route = useRoute()
const locale = computed<Locale>(() =>
  route.params.lang === 'ru' ? 'ru' : route.params.lang === 'es' ? 'es' : 'en',
)
const version = computed(() =>
  typeof route.params.version === 'string' ? route.params.version : undefined,
)
const messages = computed(() => catalogs[locale.value])
const notes = computed(() =>
  (version.value
    ? releaseNotes.filter((release) => release.version === version.value)
    : releaseNotes
  ).map((release) => ({
    ...release,
    sections: categories
      .map((category) => ({
        category,
        items: release.items.filter((item) => item.category === category),
      }))
      .filter((section) => section.items.length > 0),
  })),
)
const base = useRuntimeConfig().app.baseURL
const siteURL = 'https://adametsderschopfer.github.io/iwe-code/'
const canonical = computed(
  () => new URL(releaseNotesPath(locale.value, version.value).slice(1), siteURL).href,
)
const title = computed(() =>
  version.value
    ? `${version.value === 'unreleased' ? messages.value.releaseNotes.upcoming : version.value} — ${messages.value.releaseNotes.metaTitle}`
    : messages.value.releaseNotes.metaTitle,
)
const heading = computed(() =>
  version.value
    ? version.value === 'unreleased'
      ? messages.value.releaseNotes.upcoming
      : `IWE Code ${version.value}`
    : messages.value.releaseNotes.title,
)
function publishedDate(value: string) {
  return new Intl.DateTimeFormat(locale.value, { dateStyle: 'long', timeZone: 'UTC' }).format(
    new Date(value),
  )
}
function tagLabel(tag: string) {
  const system = tag.startsWith('macos-')
    ? 'macOS'
    : tag.startsWith('windows-')
      ? 'Windows'
      : tag.startsWith('linux-')
        ? 'Linux'
        : undefined
  return system
    ? `${messages.value.releaseNotes.github} · ${system}`
    : messages.value.releaseNotes.github
}
useHead(() => ({
  htmlAttrs: { lang: locale.value },
  title: title.value,
  link: [
    { rel: 'icon', href: `${base}icon.webp`, type: 'image/webp' },
    { rel: 'canonical', href: canonical.value },
    ...languages.map((language) => ({
      rel: 'alternate' as const,
      hreflang: language.code,
      href: new URL(releaseNotesPath(language.code, version.value).slice(1), siteURL).href,
    })),
    {
      rel: 'alternate',
      hreflang: 'x-default',
      href: new URL(releaseNotesPath('en', version.value).slice(1), siteURL).href,
    },
  ],
}))
useSeoMeta({
  description: () => messages.value.releaseNotes.description,
  ogTitle: () => title.value,
  ogDescription: () => messages.value.releaseNotes.description,
  ogType: 'website',
  ogUrl: () => canonical.value,
})
</script>

<template>
  <div>
    <SiteHeader
      :messages="messages"
      :locale="locale"
      section="releases"
      :release-version="version"
    />
    <main id="main" class="shell release-page">
      <header class="release-heading">
        <NuxtLink v-if="version" class="text-link" :to="releaseNotesPath(locale)">{{
          messages.releaseNotes.allVersions
        }}</NuxtLink>
        <p v-else class="release-eyebrow">IWE Code</p>
        <h1>{{ heading }}</h1>
        <p class="section-description">{{ messages.releaseNotes.description }}</p>
      </header>
      <nav v-if="!version" class="version-nav" :aria-label="messages.releaseNotes.versions">
        <NuxtLink
          v-for="release in notes"
          :key="release.version"
          :to="releaseNotesPath(locale, release.version)"
        >
          {{ release.version === 'unreleased' ? messages.releaseNotes.upcoming : release.version }}
        </NuxtLink>
      </nav>
      <div class="release-list">
        <article
          v-for="release in notes"
          :key="release.version"
          class="release-entry"
          :data-version="release.version"
        >
          <header class="entry-heading">
            <h2 v-if="!version">
              <NuxtLink :to="releaseNotesPath(locale, release.version)">{{
                release.version === 'unreleased'
                  ? messages.releaseNotes.upcoming
                  : `IWE Code ${release.version}`
              }}</NuxtLink>
            </h2>
            <p class="release-status">{{ messages.releaseNotes.status[release.status] }}</p>
            <time v-if="release.publishedAt" :datetime="release.publishedAt">{{
              publishedDate(release.publishedAt)
            }}</time>
          </header>
          <p v-if="release.status === 'prepared'" class="preview-note">
            {{ messages.releaseNotes.preview }}
          </p>
          <div class="entry-content">
            <section
              v-for="section in release.sections"
              :key="section.category"
              class="release-category"
            >
              <component :is="version ? 'h2' : 'h3'" class="category-title">{{
                messages.releaseNotes.categories[section.category]
              }}</component>
              <ul>
                <li v-for="item in section.items" :key="item.id">{{ item.text[locale] }}</li>
              </ul>
            </section>
          </div>
          <div v-if="release.tags.length || release.source" class="release-links">
            <a v-for="tag in release.tags" :key="tag" :href="`${repository}/releases/tag/${tag}`"
              >{{ tagLabel(tag) }}<SiteIcon name="arrow"
            /></a>
            <a v-if="release.source" :href="`${repository}/commit/${release.source}`"
              >{{ messages.releaseNotes.changes }}<SiteIcon name="arrow"
            /></a>
          </div>
        </article>
      </div>
    </main>
    <SiteFooter :messages="messages" :locale="locale" />
  </div>
</template>

<style scoped>
.release-page {
  max-width: 960px;
  padding-block: 72px 110px;
}
.release-heading {
  max-width: 700px;
}
.release-heading h1 {
  font-size: clamp(38px, 6vw, 64px);
  font-weight: 500;
  letter-spacing: -0.05em;
  line-height: 1.08;
  margin: 20px 0 0;
}
.release-eyebrow {
  color: var(--accent);
  font-size: 12px;
  margin: 0;
}
.version-nav {
  display: flex;
  flex-wrap: wrap;
  gap: 10px 22px;
  margin-block: 36px 0;
  color: var(--muted);
  font-size: 13px;
}
.release-list {
  margin-top: 56px;
}
.release-entry {
  padding-block: 36px 42px;
  border-top: 1px solid var(--line);
}
.entry-heading {
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: 10px 18px;
}
.entry-heading h2 {
  font-size: 26px;
  font-weight: 500;
  letter-spacing: -0.03em;
  margin: 0;
}
.release-status,
.entry-heading time {
  font-size: 12px;
  color: var(--muted);
  margin: 0;
}
.release-status {
  color: var(--accent);
}
.entry-heading time {
  margin-left: auto;
}
.preview-note {
  font-size: 13px;
  color: var(--muted);
  line-height: 1.6;
  max-width: 650px;
}
.entry-content {
  margin-top: 28px;
  display: grid;
  gap: 24px;
}
.category-title {
  font-size: 13px;
  color: var(--muted);
  font-weight: 500;
  margin: 0 0 12px;
}
.release-category ul {
  margin: 0;
  padding-left: 20px;
  display: grid;
  gap: 10px;
}
.release-category li {
  font-size: 15px;
  line-height: 1.65;
  padding-left: 5px;
}
.release-links {
  display: flex;
  flex-wrap: wrap;
  gap: 14px 26px;
  margin-top: 28px;
  color: var(--muted);
  font-size: 12px;
}
.release-links a {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.release-links svg {
  width: 13px;
  height: 13px;
}
@media (max-width: 600px) {
  .release-page {
    padding-block: 44px 64px;
  }
  .release-list {
    margin-top: 36px;
  }
  .entry-heading time {
    margin-left: 0;
  }
}
</style>
