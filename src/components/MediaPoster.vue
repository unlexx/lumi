<script setup lang="ts">
defineProps<{
  posterUrl: string | null
  alt: string
  rating: number | null
  watched: boolean
  progress: number | null
  placeholder?: string
}>()
</script>

<template>
  <div class="poster-wrap">
    <img
      v-if="posterUrl"
      :src="posterUrl"
      :alt="alt"
      class="poster"
      loading="lazy"
    />
    <div v-else class="poster placeholder">{{ placeholder || 'Нет постера' }}</div>

    <div v-if="rating" class="rating">★ {{ rating.toFixed(1) }}</div>
    <div v-if="watched" class="watched-badge">✓</div>
    <div v-if="progress !== null" class="progress-bar">
      <div class="progress-fill" :style="{ width: progress * 100 + '%' }"></div>
    </div>

    <slot name="overlay" />
  </div>
</template>

<style scoped>
.poster-wrap {
  position: relative;
  aspect-ratio: 2 / 3;
  border-radius: 8px;
  overflow: visible;
  background: #1e2127;
}

.poster {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
  border-radius: 8px;
}

.poster.placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  color: #666;
  font-size: 0.85rem;
}

.rating {
  position: absolute;
  top: 6px;
  right: 6px;
  background: rgba(0, 0, 0, 0.75);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 0.75rem;
  color: #ffd166;
}

.watched-badge {
  position: absolute;
  top: 6px;
  left: 6px;
  background: rgba(74, 158, 255, 0.9);
  color: #fff;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.85rem;
  font-weight: bold;
}

.progress-bar {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 4px;
  background: rgba(0, 0, 0, 0.6);
  border-radius: 0 0 8px 8px;
}

.progress-fill {
  height: 100%;
  background: #4a9eff;
  transition: width 0.3s ease;
}
</style>