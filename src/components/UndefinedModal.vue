<script setup lang="ts">
import type { UndefinedItem } from '../composables/useUndefined'

const props = defineProps<{
  item: UndefinedItem
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'play', path: string): void
  (e: 'match', item: UndefinedItem): void
}>()

function close() {
  emit('close')
}

function play() {
  emit('play', props.item.path)
}

function match() {
  emit('match', props.item)
}
</script>

<template>
  <div class="modal-backdrop" @click="close">
    <div class="modal" @click.stop>
      <button class="close" @click="close">×</button>
      <h2>{{ item.display_title }}</h2>
      <p class="file-name">{{ item.file_name }}</p>
      <div class="tags">
        <span>{{ item.media_type === 'movie' ? 'Фильм' : 'Сериал' }}</span>
        <span v-if="item.year">{{ item.year }}</span>
        <span class="no-match">Не сопоставлено</span>
      </div>
      <div class="actions">
        <button class="play-btn" @click="play">▶ Смотреть</button>
        <button class="match-btn" @click="match">Сопоставить с TMDB</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.75);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  z-index: 100;
}

.modal {
  background: #1e2127;
  border-radius: 12px;
  max-width: 600px;
  width: 100%;
  padding: 1.5rem;
  position: relative;
}

.close {
  position: absolute;
  top: 0.75rem;
  right: 0.75rem;
  background: transparent;
  border: none;
  color: #aaa;
  font-size: 1.5rem;
  cursor: pointer;
  line-height: 1;
}

.close:hover {
  color: #fff;
}

h2 {
  margin: 0 0 0.5rem 0;
  font-size: 1.3rem;
}

.file-name {
  font-family: monospace;
  font-size: 0.8rem;
  color: #666;
  word-break: break-all;
  margin: 0.5rem 0;
}

.tags {
  display: flex;
  gap: 0.5rem;
  margin: 1rem 0;
}

.tags span {
  background: #2a2e35;
  padding: 0.25rem 0.6rem;
  border-radius: 4px;
  font-size: 0.8rem;
  color: #aaa;
}

.tags .no-match {
  background: #4a2d2d;
  color: #ff8888;
}

.actions {
  display: flex;
  gap: 0.75rem;
  margin-top: 1.5rem;
}

.play-btn {
  background: #4a9eff;
  color: #fff;
  border: none;
  padding: 0.6rem 1.5rem;
  border-radius: 6px;
  font-size: 1rem;
  cursor: pointer;
}

.play-btn:hover {
  background: #3a8eef;
}

.match-btn {
  background: #2a2e35;
  color: #e6e6e6;
  border: 1px solid #3a3f47;
  padding: 0.6rem 1.5rem;
  border-radius: 6px;
  font-size: 1rem;
  cursor: pointer;
}

.match-btn:hover {
  background: #353a42;
}
</style>
