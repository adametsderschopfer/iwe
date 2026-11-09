<script setup lang="ts">
defineProps<{ name: string }>()

const paths: Record<string, string> = {
  arrow: 'M7 17 17 7M7 7h10v10',
  download: 'M12 3v12m-4-4 4 4 4-4M5 16v4h14v-4',
  spaces: 'M3 4h7v7H3zM14 4h7v7h-7zM3 15h7v6H3zM14 15h7v6h-7z',
  branch:
    'M6 7v10m12-10c0 6-12 3-12 10M9 4a3 3 0 1 1-6 0 3 3 0 0 1 6 0Zm0 16a3 3 0 1 1-6 0 3 3 0 0 1 6 0ZM21 4a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z',
  pipeline:
    'M8 12h8M7 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Zm16 0a3 3 0 1 1-6 0 3 3 0 0 1 6 0ZM12 3v3m0 12v3',
  check: 'm5 12 4 4L19 6',
  folder: 'M3 6h6l2 3h10v11H3z',
  code: 'm8 7-5 5 5 5m8-10 5 5-5 5m-3-14-2 18',
  computer: 'M3 3h18v13H3zM8 21h8m-4-5v5',
  mac: 'M16 3c-1 0-3 1-3 3 2 0 3-1 3-3ZM18 12c0-2 1-3 2-4-1-2-4-2-5-1l-2 1-2-1C7 5 3 9 4 14c1 4 3 7 5 7l4-1 3 1c2 0 4-3 5-5-2-1-3-2-3-4Z',
  windows: 'm3 5 8-1v8H3Zm10-1 8-1v9h-8ZM3 14h8v7l-8-1Zm10 0h8v9l-8-1Z',
  linux: 'M9 8V5a3 3 0 0 1 6 0v3l4 8-3 5H8l-3-5ZM9 11l3 2 3-2M8 21l-4 1m12-1 4 1',
  plus: 'M12 5v14M5 12h14',
}
</script>

<template>
  <svg
    width="24"
    height="24"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path :d="paths[name] || paths.arrow" />
  </svg>
</template>
