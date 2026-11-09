<script setup lang="ts">
import { releases, repository, type Messages } from '~/i18n'

defineProps<{ messages: Messages }>()
</script>

<template>
  <footer class="site-footer shell">
    <div class="footer-product">
      <span class="footer-brand">IWE Code</span>
      <p>{{ messages.footer.description }}</p>
    </div>
    <nav class="footer-nav" :aria-label="messages.footer.source">
      <a :href="repository">{{ messages.footer.source }}</a>
      <a :href="releases">{{ messages.footer.releases }}</a>
    </nav>
    <p class="developer">
      {{ messages.footer.developer }}
      <a href="https://vamez.ru">vamez.ru <SiteIcon name="arrow" /></a>
    </p>
  </footer>
</template>

<style scoped>
.site-footer {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 32px;
  padding-top: 38px;
  padding-bottom: 46px;
  border-top: 1px solid var(--line);
  font-size: 12px;
}
.footer-brand {
  font-size: 16px;
  font-weight: 550;
}
.footer-product p {
  color: var(--muted);
  margin-top: 8px;
}
.footer-nav {
  display: flex;
  flex-wrap: wrap;
  gap: 24px;
  color: var(--muted);
}
.developer {
  margin: 0;
  color: var(--muted);
}
.developer a {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  color: var(--text);
}
.developer svg {
  width: 13px;
  height: 13px;
}
@media (max-width: 900px) {
  .site-footer {
    flex-wrap: wrap;
  }
  .footer-product {
    width: 100%;
  }
}
</style>
