<script setup lang="ts">
import { languages, licensePath, repository, type Locale, type Messages } from '~/i18n'
import { releaseNotesPath } from '~/release-notes'

const props = withDefaults(
  defineProps<{
    messages: Messages
    locale: Locale
    section?: 'home' | 'releases' | 'license'
    releaseVersion?: string
  }>(),
  { section: 'home', releaseVersion: undefined },
)
const base = useRuntimeConfig().app.baseURL
const homeHref = computed(() => `${base}${props.locale === 'en' ? '' : `${props.locale}/`}`)
</script>

<template>
  <a class="skip-link" href="#main">{{ messages.nav.skip }}</a>
  <header class="site-header shell">
    <NuxtLink
      class="brand"
      :to="locale === 'en' ? '/' : `/${locale}/`"
      :aria-label="messages.nav.home"
    >
      <img :src="`${base}icon.webp`" width="36" height="36" alt="" />
      <span>IWE <span class="brand-code">Code</span></span>
    </NuxtLink>
    <nav class="main-nav" :aria-label="messages.nav.features">
      <a :href="section === 'home' ? '#features' : `${homeHref}#features`">{{
        messages.nav.features
      }}</a>
      <a :href="section === 'home' ? '#workflow' : `${homeHref}#workflow`">{{
        messages.nav.workflow
      }}</a>
      <NuxtLink
        :to="releaseNotesPath(locale)"
        :aria-current="section === 'releases' ? 'page' : undefined"
        >{{ messages.releaseNotes.label }}</NuxtLink
      >
      <a :href="repository">GitHub <SiteIcon name="arrow" /></a>
    </nav>
    <div class="header-actions">
      <nav class="language-nav" :aria-label="messages.nav.language">
        <NuxtLink
          v-for="language in languages"
          :key="language.code"
          :to="
            section === 'releases'
              ? releaseNotesPath(language.code, releaseVersion)
              : section === 'license'
                ? licensePath(language.code)
                : language.path
          "
          :lang="language.code"
          :aria-label="language.name"
          :aria-current="locale === language.code ? 'page' : undefined"
          >{{ language.label }}</NuxtLink
        >
      </nav>
      <a
        class="button button-small header-download"
        :href="section === 'home' ? '#download' : `${homeHref}#download`"
        >{{ messages.nav.download }}</a
      >
    </div>
  </header>
</template>

<style scoped>
.site-header {
  display: flex;
  align-items: center;
  min-height: 96px;
  gap: 32px;
  border-bottom: 1px solid var(--line);
}
.brand {
  display: inline-flex;
  gap: 10px;
  align-items: center;
  white-space: nowrap;
  font-size: 21px;
  font-weight: 650;
  letter-spacing: -0.8px;
}
.brand-code {
  color: var(--muted);
  font-weight: 450;
}
.main-nav {
  display: flex;
  gap: 20px;
  font-size: 13px;
  color: var(--muted);
}
.main-nav a {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.main-nav svg {
  width: 13px;
  height: 13px;
}
.header-actions {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 26px;
}
.language-nav {
  display: flex;
  gap: 11px;
  font-size: 11px;
  font-weight: 550;
  color: var(--muted);
}
.language-nav [aria-current] {
  color: var(--text);
}
@media (max-width: 900px) {
  .main-nav {
    display: none;
  }
  .site-header {
    min-height: 78px;
  }
}
@media (max-width: 480px) {
  .header-download {
    display: none;
  }
  .site-header {
    gap: 16px;
  }
  .header-actions {
    gap: 12px;
  }
}
</style>
