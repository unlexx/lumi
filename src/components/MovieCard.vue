<script setup lang="ts">
import type { VideoFile } from '@/types'
import MediaPoster from './MediaPoster.vue'
import ContextMenu from './ContextMenu.vue'

const props = defineProps<{
  movie: VideoFile
}>()

const emit = defineEmits<{
  (e: 'open', movie: VideoFile): void
  (e: 'play', path: string, startPosition: number | null): void
  (e: 'match', movie: VideoFile): void
  (e: 'mark-watched', movie: VideoFile, watched: boolean): void
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
        <ContextMenu>
          <template #trigger="{ toggle }">
            <button class="menu-btn" @click.stop="toggle">⋮</button>
          </template>
          <template #menu="{ close }">
            <button
              @click="
                () => {
                  emit('play', movie.path, null)
                  close()
                }
              "
            >
              Смотреть с начала
            </button>
            <button
              v-if="movie.position && movie.position > 0 && !movie.watched"
              @click="
                () => {
                  emit('play', movie.path, movie.position)
                  close()
                }
              "
            >
              Продолжить с {{ formatTime(movie.position!) }}
            </button>
            <button
              v-if="!movie.watched"
              @click="
                () => {
                  emit('mark-watched', movie, true)
                  close()
                }
              "
            >
              Пометить просмотренным
            </button>
            <button
              v-else
              @click="
                () => {
                  emit('mark-watched', movie, false)
                  close()
                }
              "
            >
              Непросмотренно
            </button>
            <button
              @click="
                () => {
                  emit('match', movie)
                  close()
                }
              "
            >
              Сопоставить
            </button>
          </template>
        </ContextMenu>
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
</style>