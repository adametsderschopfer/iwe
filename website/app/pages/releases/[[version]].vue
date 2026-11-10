<script setup lang="ts">
import { isReleaseVersion } from '~/release-notes'

definePageMeta({ validate: (route) => isReleaseVersion(route.params.version) })
</script>

<template>
  <ReleaseNotesPage />
</template>
