<script setup lang="ts">
defineProps<{
  items: unknown[]
}>()
</script>

<template>
  <div class="media-grid">
    <slot v-for="(item, index) in items" :key="index" :item="item" />
  </div>
</template>

<style scoped>
.media-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 1.25rem;
}
</style>
