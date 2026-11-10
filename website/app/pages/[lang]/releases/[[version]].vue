<script setup lang="ts">
import { isReleaseVersion } from '~/release-notes'

definePageMeta({
  validate: (route) =>
    ['ru', 'es'].includes(String(route.params.lang)) && isReleaseVersion(route.params.version),
})
</script>

<template>
  <ReleaseNotesPage />
</template>
