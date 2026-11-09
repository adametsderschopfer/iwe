<script setup lang="ts">
import type { Messages } from '~/i18n'

const { messages } = defineProps<{ messages: Messages }>()
const selected = shallowRef('workspace')
const base = useRuntimeConfig().app.baseURL
const tabs = computed(() => [
  {
    id: 'workspace',
    title: messages.preview.workspace,
    alt: messages.preview.altWorkspace,
    width: 2880,
    height: 2000,
  },
  {
    id: 'history',
    title: messages.preview.history,
    alt: messages.preview.altHistory,
    width: 2880,
    height: 1840,
  },
  {
    id: 'comparison',
    title: messages.preview.comparison,
    alt: messages.preview.altComparison,
    width: 2880,
    height: 1840,
  },
  {
    id: 'pipelines',
    title: messages.preview.pipelines,
    alt: messages.preview.altPipelines,
    width: 2160,
    height: 1440,
  },
])
</script>

<template>
  <figure class="showcase">
    <div class="showcase-toolbar">
      <div class="preview-options" role="group" :aria-label="messages.preview.label">
        <button
          v-for="tab in tabs"
          :key="tab.id"
          type="button"
          :aria-pressed="selected === tab.id"
          aria-controls="product-preview"
          @click="selected = tab.id"
        >
          {{ tab.title }}
        </button>
      </div>
      <span class="preview-product">IWE Code</span>
    </div>
    <div id="product-preview" class="preview-image">
      <template v-for="tab in tabs" :key="tab.id">
        <img
          v-show="selected === tab.id"
          :src="`${base}images/${tab.id}-en.webp`"
          :alt="tab.alt"
          :width="tab.width"
          :height="tab.height"
          :loading="tab.id === 'workspace' ? 'eager' : 'lazy'"
          :fetchpriority="tab.id === 'workspace' ? 'high' : 'auto'"
        />
      </template>
    </div>
    <figcaption>{{ messages.preview.caption }}</figcaption>
  </figure>
</template>

<style scoped>
.showcase {
  margin: 70px 0 0;
  position: relative;
}
.showcase-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 18px;
}
.preview-options {
  display: flex;
  gap: 4px;
  padding: 4px;
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 9px;
}
.preview-options button {
  padding: 8px 14px;
  border: 0;
  background: transparent;
  color: var(--muted);
  border-radius: 5px;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
}
.preview-options button[aria-pressed='true'] {
  background: var(--raised);
  color: var(--text);
  box-shadow: 0 1px 4px #0003;
}
.preview-product {
  color: var(--muted);
  font-size: 12px;
}
.preview-image {
  aspect-ratio: 1.65;
  overflow: hidden;
  border: 1px solid #444540;
  border-radius: 9px;
  background: #212121;
  box-shadow: 0 25px 90px #0004;
}
.preview-image img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  object-position: top left;
}
figcaption {
  text-align: center;
  margin-top: 20px;
  font-size: 12px;
  color: var(--muted);
}
@media (max-width: 650px) {
  .showcase {
    margin-top: 44px;
  }
  .preview-options {
    width: 100%;
    overflow-x: auto;
  }
  .preview-options button {
    flex: 1;
    padding: 8px 10px;
  }
  .preview-product {
    display: none;
  }
  .preview-image {
    aspect-ratio: 1.25;
  }
  .preview-image img {
    width: 137%;
    max-width: none;
    object-fit: cover;
  }
}
</style>
