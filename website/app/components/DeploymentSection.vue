<script setup lang="ts">
import type { Messages } from '~/i18n'

defineProps<{ messages: Messages }>()
const base = useRuntimeConfig().app.baseURL
</script>

<template>
  <section class="deployment section-space" aria-labelledby="deployment-title">
    <ScrollReveal class="deployment-heading">
      <p class="eyebrow"><SiteIcon name="server" />{{ messages.deployment.eyebrow }}</p>
      <h2 id="deployment-title" class="section-title preserve-lines">
        {{ messages.deployment.title }}
      </h2>
      <p class="section-description">{{ messages.deployment.description }}</p>
    </ScrollReveal>
    <div class="deployment-content">
      <ScrollReveal as="figure" class="deployment-preview">
        <img
          :src="`${base}images/deployment-en.webp`"
          :alt="messages.preview.altDeployment"
          width="2400"
          height="1600"
          loading="lazy"
        />
        <figcaption>{{ messages.deployment.caption }}</figcaption>
      </ScrollReveal>
      <ol class="deployment-steps">
        <li v-for="(item, index) in messages.deployment.items" :key="item.title">
          <span class="step-number" aria-hidden="true">0{{ index + 1 }}</span>
          <div>
            <h3>{{ item.title }}</h3>
            <p>{{ item.description }}</p>
          </div>
        </li>
      </ol>
    </div>
  </section>
</template>

<style scoped>
.deployment {
  border-top: 1px solid var(--line);
}
.eyebrow {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--accent);
  font-size: 12px;
  margin: 0 0 24px;
}
.eyebrow svg {
  width: 18px;
  height: 18px;
}
.deployment-heading .section-description {
  max-width: 520px;
}
.deployment-content {
  display: grid;
  grid-template-columns: minmax(0, 1.8fr) minmax(0, 1fr);
  gap: 60px;
  align-items: center;
  margin-top: 44px;
}
.deployment-preview {
  margin: 0;
}
.deployment-preview img {
  width: 100%;
  height: auto;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
}
figcaption {
  margin-top: 16px;
  color: var(--muted);
  font-size: 11px;
  line-height: 1.6;
}
.deployment-steps {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: 30px;
}
.deployment-steps li {
  display: flex;
  gap: 18px;
}
.step-number {
  color: var(--accent);
  font-size: 12px;
  line-height: 24px;
}
h3 {
  margin: 0 0 10px;
  font-size: 16px;
  font-weight: 550;
}
li p {
  margin: 0;
  font-size: 14px;
  line-height: 1.65;
  color: var(--muted);
}
@media (max-width: 1000px) {
  .deployment-content {
    gap: 32px;
  }
}
@media (max-width: 700px) {
  .deployment-content {
    grid-template-columns: 1fr;
    margin-top: 32px;
  }
}
</style>
