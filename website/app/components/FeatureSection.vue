<script setup lang="ts">
import type { Messages } from '~/i18n'

defineProps<{ messages: Messages }>()
</script>

<template>
  <section id="features" class="features section-space" aria-labelledby="features-title">
    <ScrollReveal class="feature-heading">
      <h2 id="features-title" class="section-title preserve-lines">
        {{ messages.features.title }}
      </h2>
      <p class="section-description">{{ messages.features.description }}</p>
    </ScrollReveal>
    <div class="feature-list">
      <ScrollReveal
        v-for="(feature, index) in messages.features.items"
        :key="feature.icon"
        as="article"
        class="feature"
        :delay="(index % 3) * 60"
      >
        <SiteIcon :name="feature.icon" />
        <h3>{{ feature.title }}</h3>
        <p>{{ feature.description }}</p>
      </ScrollReveal>
    </div>
  </section>
</template>

<style scoped>
.feature-heading {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 50px;
}
.feature-heading .section-description {
  max-width: 320px;
  margin: 0 0 4px;
}
.feature-list {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 44px;
  margin-top: 56px;
}
.feature {
  padding-top: 24px;
  border-top: 1px solid var(--line-strong);
}
.feature > svg {
  color: var(--accent);
  width: 25px;
  height: 25px;
  margin-bottom: 22px;
}
.feature h3 {
  font-size: 16px;
  font-weight: 550;
  margin: 0 0 13px;
}
.feature p {
  font-size: 14px;
  line-height: 1.65;
  color: var(--muted);
  margin: 0;
  max-width: 420px;
}
@media (max-width: 1000px) {
  .feature-list {
    gap: 32px;
  }
}
@media (max-width: 700px) {
  .feature-heading {
    display: block;
  }
  .feature-heading .section-description {
    margin-top: 22px;
    max-width: 450px;
  }
  .feature-list {
    grid-template-columns: 1fr;
    gap: 28px;
    margin-top: 36px;
  }
}
</style>
