<script setup lang="ts">
import type { VideoFile } from '@/types'
import MediaPoster from './MediaPoster.vue'

const props = defineProps<{
  movie: VideoFile
  menuOpen: boolean
}>()

const emit = defineEmits<{
  (e: 'open', movie: VideoFile): void
  (e: 'play', path: string, startPosition: number | null): void
  (e: 'match', movie: VideoFile): void
  (e: 'mark-watched', movie: VideoFile, watched: boolean): void
  (e: 'toggle-menu', path: string): void
}>()

function progressPercent(): number | null {
  if (props.movie.watched) return null
  const p = props.movie.position
  const d = props.movie.duration
  if (!p || !d || d <= 0) return null
  const ratio = p / d
  if (ratio <= 0.01) return null
  if (ratio >= 0.95) return null
  return ratio
}

function formatTime(seconds: number): string {
  if (!isFinite(seconds) || seconds < 0) return '0:00'
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = Math.floor(seconds % 60)
  if (h > 0) {
    return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
  }
  return `${m}:${String(s).padStart(2, '0')}`
}

function handlePlayFromStart() {
  emit('play', props.movie.path, null)
  emit('toggle-menu', props.movie.path)
}

function handleResume() {
  emit('play', props.movie.path, props.movie.position)
  emit('toggle-menu', props.movie.path)
}

function handleMarkWatched() {
  emit('mark-watched', props.movie, !props.movie.watched)
  emit('toggle-menu', props.movie.path)
}

function handleMatch() {
  emit('match', props.movie)
  emit('toggle-menu', props.movie.path)
}
</script>

<template>
  <article class="card" @click="emit('open', movie)">
    <MediaPoster
      :poster-url="movie.tmdb?.poster_local || null"
      :alt="movie.tmdb?.title || movie.parsed.title"
      :rating="movie.tmdb?.rating ?? null"
      :watched="movie.watched"
      :progress="progressPercent()"
      placeholder="Нет постера"
    >
      <template #overlay>
        <button class="menu-btn" @click.stop="emit('toggle-menu', movie.path)">⋮</button>
        <div v-if="menuOpen" class="context-menu" @click.stop>
          <button @click="handlePlayFromStart">Смотреть с начала</button>
          <button
            v-if="movie.position && movie.position > 0 && !movie.watched"
            @click="handleResume"
          >
            Продолжить с {{ formatTime(movie.position) }}
          </button>
          <button v-if="!movie.watched" @click="handleMarkWatched">
            Пометить просмотренным
          </button>
          <button v-else @click="handleMarkWatched">Непросмотренно</button>
          <button @click="handleMatch">Сопоставить</button>
        </div>
      </template>
    </MediaPoster>

    <div class="card-title">{{ movie.tmdb?.title || movie.parsed.title }}</div>
    <div class="card-year">{{ movie.parsed.year || '' }}</div>
  </article>
</template>

<style scoped>
.card {
  cursor: pointer;
  transition: transform 0.15s ease;
}

.card:hover {
  transform: translateY(-4px);
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

.menu-btn {
  position: absolute;
  bottom: 6px;
  right: 6px;
  background: rgba(0, 0, 0, 0.6);
  color: #fff;
  border: none;
  border-radius: 50%;
  width: 28px;
  height: 28px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1rem;
  line-height: 1;
  opacity: 0;
  transition:
    opacity 0.15s ease,
    background 0.15s ease;
  z-index: 5;
}

.card:hover .menu-btn {
  opacity: 1;
}

.menu-btn:hover {
  background: rgba(0, 0, 0, 0.9);
}

.context-menu {
  position: absolute;
  bottom: 38px;
  right: 6px;
  background: #1e2127;
  border: 1px solid #3a3f47;
  border-radius: 6px;
  padding: 0.25rem 0;
  min-width: 120px;
  max-width: 240px;
  z-index: 10;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
}

.context-menu button {
  display: block;
  width: 100%;
  background: transparent;
  border: none;
  color: #e6e6e6;
  text-align: left;
  padding: 0.5rem 1rem;
  cursor: pointer;
  font-size: 0.85rem;
  white-space: normal;
  word-break: break-word;
}

.context-menu button:hover {
  background: #2a2e35;
}
</style>