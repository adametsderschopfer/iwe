<script setup lang="ts">
import { releases, type Messages } from '~/i18n'
import {
  detectPlatform,
  type DetectedPlatform,
  type DesktopSystem,
  type DesktopArchitecture,
} from '~/utils/platform'

defineProps<{ messages: Messages }>()
const version = '0.1.11'
const downloadBase = 'https://github.com/adametsderschopfer/iwe-code/releases/download'
const platforms = [
  {
    id: 'macos',
    name: 'macOS',
    icon: 'mac',
    description: 'mac',
    files: [
      { label: 'Apple Silicon', slug: 'macos-arm64.dmg', architecture: 'arm64' },
      { label: 'Intel', slug: 'macos-x64.dmg', architecture: 'x64' },
    ],
  },
  {
    id: 'windows',
    name: 'Windows',
    icon: 'windows',
    description: 'windows',
    files: [
      { label: 'x64', slug: 'windows-x64.zip', architecture: 'x64' },
      { label: 'x86', slug: 'windows-x86.zip', architecture: 'x86' },
    ],
  },
  {
    id: 'linux',
    name: 'Linux',
    icon: 'linux',
    description: 'linux',
    files: [
      { label: 'x64', slug: 'linux-x64.tar.gz', architecture: 'x64' },
      { label: 'x86', slug: 'linux-x86.tar.gz', architecture: 'x86' },
    ],
  },
] as const

const detected = shallowRef<DetectedPlatform>({ system: undefined, architecture: undefined })
const orderedPlatforms = computed(() =>
  [...platforms].sort(
    (left, right) =>
      Number(right.id === detected.value.system) - Number(left.id === detected.value.system),
  ),
)
let disposed = false

function isPriority(system: DesktopSystem, architecture: DesktopArchitecture) {
  if (!detected.value.system) return system === 'macos' && architecture === 'arm64'
  if (system !== detected.value.system) return false
  if (detected.value.architecture) return architecture === detected.value.architecture
  return system === 'macos' && architecture === 'arm64'
}

onBeforeUnmount(() => {
  disposed = true
})
onMounted(async () => {
  const browser = navigator as Navigator & {
    userAgentData?: {
      platform?: string
      mobile?: boolean
      getHighEntropyValues?: (
        hints: string[],
      ) => Promise<{ platform?: string; architecture?: string; bitness?: string; mobile?: boolean }>
    }
  }
  const input = {
    userAgent: browser.userAgent,
    platform: browser.platform,
    maxTouchPoints: browser.maxTouchPoints,
  }
  const basicHints = {
    platform: browser.userAgentData?.platform,
    mobile: browser.userAgentData?.mobile,
  }
  detected.value = detectPlatform({ ...input, hints: basicHints })
  try {
    const hints = await browser.userAgentData?.getHighEntropyValues?.([
      'architecture',
      'bitness',
      'platform',
    ])
    if (hints && !disposed)
      detected.value = detectPlatform({ ...input, hints: { ...basicHints, ...hints } })
  } catch {
    // Keep the platform fallback when the browser withholds architecture information.
  }
})
</script>

<template>
  <section id="download" class="download-section section-space" aria-labelledby="download-title">
    <ScrollReveal class="download-heading">
      <p class="release-note"><span />{{ messages.download.early }}</p>
      <h2 id="download-title" class="section-title preserve-lines">
        {{ messages.download.title }}
      </h2>
      <p class="section-description">{{ messages.download.description }}</p>
    </ScrollReveal>
    <div class="platform-list" :aria-label="messages.download.architecture">
      <ScrollReveal
        v-for="(platform, index) in orderedPlatforms"
        :key="platform.name"
        as="article"
        class="platform"
        :class="{ 'platform-detected': detected.system === platform.id }"
        :data-platform="platform.id"
        :delay="index * 60"
      >
        <div class="platform-top">
          <SiteIcon :name="platform.icon" class="platform-icon" /><span
            v-if="detected.system === platform.id"
            class="detected-badge"
            ><SiteIcon name="check" />{{ messages.download.detected }}</span
          >
        </div>
        <h3>{{ platform.name }}</h3>
        <p>{{ messages.download[platform.description] }}</p>
        <div class="platform-downloads">
          <a
            v-for="file in platform.files"
            :key="file.slug"
            :href="`${downloadBase}/${version}/iwe-${version}-${file.slug}`"
            :aria-label="`${messages.download.download} IWE Code ${platform.name} ${file.label}`"
            :class="{ 'download-priority': isPriority(platform.id, file.architecture) }"
            >{{ file.label }}<SiteIcon name="download"
          /></a>
        </div>
        <p
          v-if="platform.id === 'macos' && (!detected.architecture || !detected.system)"
          class="platform-hint"
        >
          {{ messages.download.siliconHint }}
        </p>
        <p
          v-else-if="
            platform.id === detected.system &&
            detected.architecture === 'arm64' &&
            platform.id !== 'macos'
          "
          class="platform-hint"
        >
          {{ messages.download.armUnavailable }}
        </p>
      </ScrollReveal>
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
  gap: 14px;
  max-width: 1000px;
  margin: 60px auto 28px;
}
.platform {
  padding: 28px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
}
.platform-detected {
  background: #242a1e;
  border-color: #7a8969;
}
.platform-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 27px;
}
.detected-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: #d1dfc2;
  font-size: 10px;
  white-space: nowrap;
}
.detected-badge svg {
  width: 12px;
  height: 12px;
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
.platform-downloads .download-priority {
  color: #181b15;
  background: #e7ebdf;
  border-color: #e7ebdf;
  font-weight: 550;
}
.platform-downloads .download-priority:hover {
  color: #181b15;
  background: #fff;
}
.platform .platform-hint {
  font-size: 11px;
  line-height: 1.6;
  color: var(--muted);
  margin: 18px 0 0;
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
    padding: 24px;
  }
  .platform h3 {
    margin: 20px 0 6px;
  }
  .platform p {
    margin: 0 0 18px;
  }
}
</style>
