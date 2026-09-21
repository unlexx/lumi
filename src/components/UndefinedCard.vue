<script setup lang="ts">
import type { UndefinedItem } from '@/types'

defineProps<{
  item: UndefinedItem
}>()

defineEmits<{
  (e: 'click', item: UndefinedItem): void
}>()
</script>

<template>
  <article class="undefined-card" @click="$emit('click', item)">
    <div class="poster-wrap">
      <div class="poster placeholder">?</div>
    </div>
    <div class="card-title">{{ item.display_title }}</div>
    <div class="card-year">
      {{ item.year || '—' }} · {{ item.media_type === 'movie' ? 'Фильм' : 'Сериал' }}
    </div>
  </article>
</template>

<style scoped>
.undefined-card {
  cursor: pointer;
  transition: transform 0.15s ease;
}

.undefined-card:hover {
  transform: translateY(-4px);
}

.poster-wrap {
  position: relative;
  aspect-ratio: 2 / 3;
  border-radius: 8px;
  overflow: hidden;
  background: #1e2127;
}

.poster.placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #444;
  font-size: 3rem;
  font-weight: 300;
  background: #1a1d22;
}

.card-title {
  margin-top: 0.5rem;
  font-size: 0.9rem;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.card-year {
  font-size: 0.8rem;
  color: #888;
}
</style>
