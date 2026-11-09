<script setup lang="ts">
import { releases, type Messages } from '~/i18n'

defineProps<{ messages: Messages }>()
const version = '0.1.11'
const downloadBase = 'https://github.com/adametsderschopfer/iwe-code/releases/download'
const platforms = [
  {
    name: 'macOS',
    icon: 'mac',
    description: 'mac',
    files: [
      { label: 'Apple Silicon', slug: 'macos-arm64.dmg' },
      { label: 'Intel', slug: 'macos-x64.dmg' },
    ],
  },
  {
    name: 'Windows',
    icon: 'windows',
    description: 'windows',
    files: [
      { label: 'x64', slug: 'windows-x64.zip' },
      { label: 'x86', slug: 'windows-x86.zip' },
    ],
  },
  {
    name: 'Linux',
    icon: 'linux',
    description: 'linux',
    files: [
      { label: 'x64', slug: 'linux-x64.tar.gz' },
      { label: 'x86', slug: 'linux-x86.tar.gz' },
    ],
  },
] as const
</script>

<template>
  <section id="download" class="download-section section-space" aria-labelledby="download-title">
    <div class="download-heading">
      <p class="release-note"><span />{{ messages.download.early }}</p>
      <h2 id="download-title" class="section-title preserve-lines">
        {{ messages.download.title }}
      </h2>
      <p class="section-description">{{ messages.download.description }}</p>
    </div>
    <div class="platform-list" :aria-label="messages.download.architecture">
      <article v-for="platform in platforms" :key="platform.name" class="platform">
        <SiteIcon :name="platform.icon" class="platform-icon" />
        <h3>{{ platform.name }}</h3>
        <p>{{ messages.download[platform.description] }}</p>
        <div class="platform-downloads">
          <a
            v-for="file in platform.files"
            :key="file.slug"
            :href="`${downloadBase}/${version}/iwe-${version}-${file.slug}`"
            :aria-label="`${messages.download.download} IWE Code ${platform.name} ${file.label}`"
            >{{ file.label }}<SiteIcon name="download"
          /></a>
        </div>
      </article>
    </div>
    <a class="release-link" :href="releases"
      >{{ messages.download.release }} <SiteIcon name="arrow"
    /></a>
  </section>
</template>

<style scoped>
.download-section {
  scroll-margin-top: 40px;
  border-top: 1px solid var(--line);
}
.download-heading {
  text-align: center;
  max-width: 650px;
  margin: auto;
}
.download-heading .section-description {
  max-width: 490px;
  margin-inline: auto;
}
.release-note {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  margin: 0 0 26px;
  color: var(--muted);
}
.release-note span {
  width: 5px;
  height: 5px;
  background: var(--accent);
  border-radius: 50%;
}
.platform-list {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  max-width: 900px;
  margin: 60px auto 28px;
  border: 1px solid var(--line);
  border-radius: 12px;
  overflow: hidden;
  background: var(--surface);
}
.platform {
  padding: 34px;
  border-right: 1px solid var(--line);
}
.platform:last-child {
  border: 0;
}
.platform-icon {
  width: 27px;
  height: 27px;
}
.platform h3 {
  margin: 25px 0 7px;
  font-size: 20px;
  font-weight: 550;
}
.platform p {
  color: var(--muted);
  font-size: 12px;
  margin-bottom: 26px;
}
.platform-downloads {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.platform-downloads a {
  display: inline-flex;
  gap: 10px;
  align-items: center;
  border: 1px solid var(--line);
  border-radius: 6px;
  padding: 9px 12px;
  font-size: 12px;
}
.platform-downloads a:hover {
  background: var(--raised);
}
.platform-downloads svg {
  width: 14px;
  height: 14px;
}
.release-link {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--muted);
  text-align: center;
}
.release-link svg {
  flex-shrink: 0;
  width: 14px;
  height: 14px;
}
@media (max-width: 650px) {
  .platform-list {
    grid-template-columns: 1fr;
    margin-top: 38px;
  }
  .platform {
    display: grid;
    grid-template-columns: 32px 1fr;
    column-gap: 16px;
    padding: 24px;
    border-right: 0;
    border-bottom: 1px solid var(--line);
  }
  .platform-icon {
    grid-row: span 2;
  }
  .platform h3 {
    margin: 0 0 6px;
  }
  .platform p {
    margin: 0 0 18px;
  }
  .platform-downloads {
    grid-column: 2;
  }
}
</style>
