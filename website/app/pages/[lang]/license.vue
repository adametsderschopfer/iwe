<script setup lang="ts">
definePageMeta({ validate: (route) => ['ru', 'es'].includes(String(route.params.lang)) })
</script>

<template>
  <LicensePage />
</template>
