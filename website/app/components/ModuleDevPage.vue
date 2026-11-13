<script setup lang="ts">
import { catalogs, languages, modulesPath, repository, type Locale } from '~/i18n'

const route = useRoute()
const locale = computed<Locale>(() =>
  route.params.lang === 'ru' ? 'ru' : route.params.lang === 'es' ? 'es' : 'en',
)
const messages = computed(() => catalogs[locale.value])
const base = useRuntimeConfig().app.baseURL
const siteURL = 'https://adametsderschopfer.github.io/iwe-code/'
const canonical = computed(() => new URL(modulesPath(locale.value).slice(1), siteURL).href)
useHead(() => ({
  htmlAttrs: { lang: locale.value },
  title: messages.value.modules.metaTitle,
  link: [
    { rel: 'icon', href: `${base}icon.webp`, type: 'image/webp' },
    { rel: 'canonical', href: canonical.value },
    ...languages.map((language) => ({
      rel: 'alternate' as const,
      hreflang: language.code,
      href: new URL(modulesPath(language.code).slice(1), siteURL).href,
    })),
    { rel: 'alternate', hreflang: 'x-default', href: new URL('modules/', siteURL).href },
  ],
}))
useSeoMeta({
  description: () => messages.value.modules.description,
  ogTitle: () => messages.value.modules.metaTitle,
  ogDescription: () => messages.value.modules.description,
  ogType: 'website',
  ogUrl: () => canonical.value,
})

const manifestExample = `{
  "id": "hello-module",
  "name": { "en": "Hello module", "ru": "Модуль приветствия" },
  "description": { "en": "Shows a friendly panel." },
  "version": "1.0.0",
  "api": 1,
  "minAppVersion": "1.0.8",
  "entry": { "macos-aarch64": "hello-module", "linux-x86_64": "hello-module" },
  "permissions": [],
  "placement": "bottom"
}`

const packageExample = `hello-module/
├── manifest.json
└── binaries/
    ├── macos-aarch64/hello-module
    └── linux-x86_64/hello-module`

const protocolExample = `{"type":"initialize","api":1,"appVersion":"1.0.8","language":"en","installPath":"/…/modules/hello-module/1.0.0"}
{"type":"panel","content":{"type":"column","children":[{"type":"text","content":"Hello!"}]}}
{"type":"event","id":"click","action":{"type":"click"}}
{"type":"notify","level":"info","message":"Done"}
{"type":"shutdown"}`

const buildExample = `cargo build --release --example hello-module
mkdir -p hello-module/binaries/macos-aarch64
cp target/release/examples/hello-module hello-module/binaries/macos-aarch64/
cp manifest.json hello-module/
tar -czf hello-module.iwe-module -C hello-module manifest.json binaries`
</script>

<template>
  <div>
    <SiteHeader :messages="messages" :locale="locale" section="modules" />
    <main id="main" class="shell module-page">
      <h1>{{ messages.modules.title }}</h1>
      <p class="section-description">{{ messages.modules.description }}</p>
      <p class="module-intro">{{ messages.modules.intro }}</p>

      <section class="module-section">
        <h2>{{ messages.modules.conceptTitle }}</h2>
        <ul class="module-list">
          <li v-for="item in messages.modules.concept" :key="item">{{ item }}</li>
        </ul>
      </section>

      <section class="module-section">
        <h2>{{ messages.modules.manifestTitle }}</h2>
        <p>{{ messages.modules.manifestIntro }}</p>
        <dl class="module-table">
          <template v-for="field in messages.modules.manifestFields" :key="field.field">
            <dt>
              <code>{{ field.field }}</code>
            </dt>
            <dd>{{ field.description }}</dd>
          </template>
        </dl>
        <pre class="module-code">{{ manifestExample }}</pre>
      </section>

      <section class="module-section">
        <h2>{{ messages.modules.packageTitle }}</h2>
        <p>{{ messages.modules.packageIntro }}</p>
        <pre class="module-code">{{ packageExample }}</pre>
      </section>

      <section class="module-section">
        <h2>{{ messages.modules.protocolTitle }}</h2>
        <p>{{ messages.modules.protocolIntro }}</p>
        <h3>{{ messages.modules.hostTitle }}</h3>
        <dl class="module-table">
          <template v-for="entry in messages.modules.hostMessages" :key="entry.name">
            <dt>
              <code>{{ entry.name }}</code>
            </dt>
            <dd>{{ entry.description }}</dd>
          </template>
        </dl>
        <h3>{{ messages.modules.moduleTitle }}</h3>
        <dl class="module-table">
          <template v-for="entry in messages.modules.moduleMessages" :key="entry.name">
            <dt>
              <code>{{ entry.name }}</code>
            </dt>
            <dd>{{ entry.description }}</dd>
          </template>
        </dl>
        <pre class="module-code">{{ protocolExample }}</pre>
      </section>

      <section class="module-section">
        <h2>{{ messages.modules.widgetsTitle }}</h2>
        <p>{{ messages.modules.widgetsIntro }}</p>
        <dl class="module-table">
          <template v-for="widget in messages.modules.widgets" :key="widget.name">
            <dt>
              <code>{{ widget.name }}</code>
            </dt>
            <dd>{{ widget.description }}</dd>
          </template>
        </dl>
      </section>

      <section class="module-section">
        <h2>{{ messages.modules.permissionsTitle }}</h2>
        <p>{{ messages.modules.permissionsIntro }}</p>
        <dl class="module-table">
          <template v-for="permission in messages.modules.permissions" :key="permission.name">
            <dt>
              <code>{{ permission.name }}</code>
            </dt>
            <dd>{{ permission.description }}</dd>
          </template>
        </dl>
      </section>

      <section class="module-section">
        <h2>{{ messages.modules.buildTitle }}</h2>
        <p>{{ messages.modules.buildIntro }}</p>
        <pre class="module-code">{{ buildExample }}</pre>
      </section>

      <section class="module-section">
        <h2>{{ messages.modules.installTitle }}</h2>
        <p>{{ messages.modules.installIntro }}</p>
      </section>

      <section class="module-section">
        <h2>{{ messages.modules.exampleTitle }}</h2>
        <p>{{ messages.modules.exampleIntro }}</p>
        <a class="text-link module-source" :href="`${repository}/tree/main/examples/hello-module`"
          >{{ messages.modules.viewExample }} <SiteIcon name="arrow"
        /></a>
      </section>
    </main>
    <SiteFooter :messages="messages" :locale="locale" />
  </div>
</template>

<style scoped>
.module-page {
  max-width: 960px;
  padding-block: 72px 110px;
}
h1 {
  font-size: clamp(36px, 6vw, 60px);
  font-weight: 500;
  letter-spacing: -0.05em;
  margin: 0;
}
.module-intro {
  margin-top: 24px;
  max-width: 640px;
  font-size: 15px;
  line-height: 1.7;
}
.module-section {
  border-top: 1px solid var(--line);
  margin-top: 48px;
  padding-top: 32px;
}
.module-section h2 {
  font-size: 22px;
  font-weight: 550;
  letter-spacing: -0.02em;
  margin: 0;
}
.module-section h3 {
  font-size: 14px;
  font-weight: 550;
  color: var(--muted);
  margin: 24px 0 0;
}
.module-section p {
  margin: 12px 0 0;
  max-width: 720px;
  color: var(--muted);
  line-height: 1.7;
}
.module-list {
  margin: 16px 0 0;
  padding-left: 20px;
  color: var(--muted);
  line-height: 1.8;
}
.module-table {
  display: grid;
  grid-template-columns: minmax(140px, auto) 1fr;
  gap: 10px 24px;
  margin: 20px 0 0;
}
.module-table dt {
  font-size: 13px;
}
.module-table dt code {
  font-size: 12px;
  color: var(--text);
}
.module-table dd {
  margin: 0;
  font-size: 13px;
  color: var(--muted);
  line-height: 1.6;
}
.module-code {
  margin: 20px 0 0;
  padding: 18px 20px;
  overflow-x: auto;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--raised);
  font-size: 12px;
  line-height: 1.7;
  white-space: pre;
}
.module-source {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-top: 20px;
  font-size: 13px;
}
.module-source svg {
  width: 13px;
  height: 13px;
}
@media (max-width: 600px) {
  .module-page {
    padding-block: 44px 64px;
  }
  .module-table {
    grid-template-columns: 1fr;
    gap: 4px;
  }
  .module-table dd {
    margin-bottom: 10px;
  }
}
</style>
