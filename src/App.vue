<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import Settings from './views/Settings.vue'
import { listen } from '@tauri-apps/api/event'
import MediaGrid from './components/MediaGrid.vue'
import UndefinedCard from './components/UndefinedCard.vue'
import UndefinedModal from './components/UndefinedModal.vue'
import type { VideoFile, TvShow, TmdbSearchResult, MatchResult, UndefinedItem } from '@/types'
import { useLibrary } from '@/composables/useLibrary'
import { usePosters } from '@/composables/usePosters'
import { usePlayer } from '@/composables/usePlayer'
import { useWatched } from '@/composables/useWatched'
import { useContinueWatching } from '@/composables/useContinueWatching'
import { useKeyboard } from '@/composables/useKeyboard'
import { useUndefined } from '@/composables/useUndefined'
import MovieCard from '@/components/MovieCard.vue'
import ShowCard from '@/components/ShowCard.vue'
import MatchModal from './components/MatchModal.vue'
import MovieModal from './components/MovieModal.vue'
import '@/styles/modal.css'

const { library, loading, error, scanProgress, refresh } = useLibrary()
const { load: loadPosters } = usePosters(library)
const { play } = usePlayer()
const { continueWatching, load: loadContinueWatching } = useContinueWatching(library)
const { markMovie, markShow } = useWatched(loadContinueWatching)
const { items: undefinedItems, load: loadUndefined, remove: removeUndefined } = useUndefined()

const currentView = ref<'library' | 'settings'>('library')
const selectedVideo = ref<VideoFile | null>(null)
const selectedShow = ref<TvShow | null>(null)

useKeyboard({ currentView, selectedVideo, selectedShow })

const matchTarget = ref<{
  path: string
  media_type: 'movie' | 'tv_shows'
  title: string
  uid?: string
} | null>(null)

async function refreshAll() {
  await refresh()
  await loadPosters()
  await loadContinueWatching()
  await loadUndefined()
}

onMounted(async () => {
  await listen('scan_progress', (event) => {
    scanProgress.value = event.payload as {
      current: number
      total: number
      folder: string
    }
  })

  await listen('watch_status_updated', async (event) => {
    const { path, watched, position, duration } = event.payload as {
      path: string
      watched: boolean
      position: number
      duration: number
    }
    // Фильм?
    const movie = library.value.movies.find((m) => m.path === path)
    if (movie) {
      movie.watched = watched
      movie.position = position
      movie.duration = duration
    } else {
      for (const show of library.value.tv_shows) {
        for (const season of show.seasons) {
          const ep = season.episodes.find((e) => e.path === path)
          if (ep) {
            ep.watched = watched
            ep.position = position
            ep.duration = duration
            break
          }
        }
      }
    }
    await loadContinueWatching()
  })
  refreshAll()
})

function openMovie(video: VideoFile) {
  selectedVideo.value = video
}

function closeMovie() {
  selectedVideo.value = null
}

function openShow(show: TvShow) {
  selectedShow.value = show
}

function closeShow() {
  selectedShow.value = null
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

function openMatchModal(target: {
  path: string
  media_type: 'movie' | 'tv_shows'
  title: string
  uid?: string
}) {
  matchTarget.value = target
}

function openMatchModalForMovie(movie: VideoFile) {
  openMatchModal({
    path: movie.path,
    media_type: 'movie',
    title: movie.tmdb?.title || movie.parsed.title,
    uid: movie.uid
  })
}

function openMatchModalForShow(show: TvShow) {
  openMatchModal({
    path: '',
    media_type: 'tv_shows',
    title: show.title,
    uid: undefined
  })
}

function matchUndefined(item: UndefinedItem) {
  matchTarget.value = {
    path: item.path,
    media_type: item.media_type,
    title: item.display_title,
    uid: item.uid
  }
  selectedUndefined.value = null
}

async function applyMatch(result: TmdbSearchResult) {
  const target = matchTarget.value
  if (!target) {
    console.error('No match target set')
    return
  }

  try {
    await invoke<MatchResult>('apply_tmdb_match', {
      tmdbId: result.id,
      mediaType: target.media_type,
      uid: target.uid ?? null
    })

    if (target.uid) {
      removeUndefined(target.uid)
    }

    await refreshAll()
    matchTarget.value = null
  } catch (e) {
    console.error('Apply match error:', e)
    error.value = String(e)
  }
}

const selectedUndefined = ref<UndefinedItem | null>(null)

function openUndefined(item: UndefinedItem) {
  selectedUndefined.value = item
}

function closeUndefined() {
  selectedUndefined.value = null
}

function playUndefined(path: string) {
  play(path)
  selectedUndefined.value = null
}
</script>

<template>
  <main class="app">
    <header class="toolbar">
      <div class="header-left">
        <button
          v-if="currentView === 'settings'"
          class="back-arrow"
          @click="currentView = 'library'"
          title="Назад (Backspace)"
        >
          ←
        </button>
        <h1>Lumi</h1>
      </div>
      <nav class="tabs">
        <button :class="{ active: currentView === 'library' }" @click="currentView = 'library'">
          Библиотека
        </button>
        <button :class="{ active: currentView === 'settings' }" @click="currentView = 'settings'">
          Настройки
        </button>
        <button
          v-if="currentView === 'library'"
          :disabled="loading"
          @click="refreshAll"
          title="Обновить библиотеку"
        >
          ↻
        </button>
      </nav>
    </header>

    <Settings v-if="currentView === 'settings'" />

    <div v-else>
      <div v-if="loading" class="loading">
        <div class="spinner"></div>
        <p v-if="scanProgress" class="progress-text">
          Сканирование папки {{ scanProgress.current }} из {{ scanProgress.total }}
        </p>
        <p v-else class="progress-text">Подготовка...</p>
        <p v-if="scanProgress" class="progress-folder">{{ scanProgress.folder }}</p>
      </div>
      <template v-else>
        <p v-if="error" class="error">{{ error }}</p>
        <section v-if="continueWatching.length" class="section">
          <h2>Продолжить просмотр</h2>
          <div class="continue-row">
            <article
              v-for="item in continueWatching"
              :key="item.path"
              class="continue-card"
              @click="play(item.path, item.position)"
            >
              <div class="continue-poster-wrap">
                <img
                  v-if="item.poster_url"
                  :src="item.poster_url"
                  :alt="item.title"
                  class="poster"
                  loading="lazy"
                />
                <div v-else class="poster placeholder">—</div>
                <div class="progress-bar">
                  <div class="progress-fill" :style="{ width: item.progress * 100 + '%' }"></div>
                </div>
              </div>
              <div class="card-title">{{ item.title }}</div>
              <div class="card-year">
                {{ formatTime(item.position) }} / {{ formatTime(item.duration) }}
              </div>
            </article>
          </div>
        </section>
        <!-- Фильмы -->
        <section v-if="library.movies.length" class="section">
          <h2>Фильмы</h2>
          <div class="grid">
            <MovieCard
              v-for="movie in library.movies"
              :key="movie.path"
              :movie="movie"
              @open="openMovie"
              @play="play"
              @match="openMatchModalForMovie"
              @mark-watched="markMovie"
            />
          </div>
        </section>
        <!-- Сериалы -->
        <section v-if="library.tv_shows.length" class="section">
          <h2>Сериалы</h2>
          <div class="grid">
            <ShowCard
              v-for="show in library.tv_shows"
              :key="show.title"
              :show="show"
              @open="openShow"
              @play="play"
              @match="openMatchModalForShow"
              @mark-watched="markShow"
            />
          </div>
        </section>
        <section v-if="undefinedItems.length" class="section">
          <h2>Неопределённое ({{ undefinedItems.length }})</h2>
          <MediaGrid :items="undefinedItems">
            <template #default="{ item }">
              <UndefinedCard :item="item" @click="openUndefined" />
            </template>
          </MediaGrid>
        </section>
        <p v-if="!error && !library.movies.length && !library.tv_shows.length" class="status">
          Библиотека пуста. Добавьте папки в настройках.
        </p>
      </template>
    </div>
    <!-- Модалка фильма -->
    <MovieModal v-if="selectedVideo" :movie="selectedVideo" @close="closeMovie" @play="play" />
    <!-- Модалка сериала -->
    <ShowModal v-if="selectedShow" :show="selectedShow" @close="closeShow" @play="play" />
  </main>
  <MatchModal v-if="matchTarget" :target="matchTarget" @close="matchTarget = null" @apply="applyMatch" />
  <UndefinedModal
    v-if="selectedUndefined"
    :item="selectedUndefined"
    @close="closeUndefined"
    @play="playUndefined"
    @match="matchUndefined"
  />
</template>

<style>
* {
  box-sizing: border-box;
}

body {
  margin: 0;
  background: #14161a;
  color: #e6e6e6;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}

.app {
  min-height: 100vh;
  padding: 1.5rem 2rem;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 1.5rem;
}

.toolbar h1 {
  margin: 0;
  font-size: 1.5rem;
  letter-spacing: 0.05em;
}

.toolbar button {
  background: #2a2e35;
  color: #e6e6e6;
  border: 1px solid #3a3f47;
  padding: 0.5rem 1rem;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.9rem;
}

.toolbar button:hover:not(:disabled) {
  background: #353a42;
}

.toolbar button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.error {
  color: #ff6b6b;
  padding: 0.5rem 0;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 1.25rem;
}
.poster {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
  border-radius: 8px;
}

.tabs {
  display: flex;
  gap: 0.5rem;
}
.tabs button {
  background: transparent;
  border: none;
  color: #888;
  padding: 0.5rem 1rem;
  cursor: pointer;
  border-radius: 6px;
  font-size: 0.9rem;
}
.tabs button:hover {
  color: #e6e6e6;
}
.tabs button.active {
  background: #2a2e35;
  color: #e6e6e6;
}
.header-left {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.back-arrow {
  background: transparent;
  border: none;
  color: #aaa;
  font-size: 1.5rem;
  cursor: pointer;
  padding: 0 0.5rem;
  line-height: 1;
  transition: color 0.15s ease;
}
.back-arrow:hover {
  color: #fff;
}
.section {
  margin-bottom: 2rem;
}

.section h2 {
  margin: 1rem 0 1rem;
  font-size: 1.1rem;
  color: #ccc;
  font-weight: 500;
}

.status {
  color: #888;
  padding: 1rem 0;
}
.loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 4rem 0;
  gap: 1rem;
}

.spinner {
  width: 40px;
  height: 40px;
  border: 3px solid #2a2e35;
  border-top-color: #4a9eff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.progress-text {
  margin: 0;
  color: #ccc;
  font-size: 0.95rem;
}

.progress-folder {
  margin: 0;
  color: #666;
  font-size: 0.8rem;
  font-family: monospace;
  max-width: 500px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Продолжить просмотр — горизонтальная прокрутка */
.continue-row {
  display: flex;
  gap: 1rem;
  overflow-x: auto;
  padding-bottom: 0.5rem;
  scroll-behavior: smooth;
}

.continue-row::-webkit-scrollbar {
  height: 6px;
}

.continue-row::-webkit-scrollbar-track {
  background: #1e2127;
  border-radius: 3px;
}

.continue-row::-webkit-scrollbar-thumb {
  background: #3a3f47;
  border-radius: 3px;
}

.continue-row::-webkit-scrollbar-thumb:hover {
  background: #4a5058;
}

.continue-card {
  flex: 0 0 160px; /* фиксированная ширина карточки */
  cursor: pointer;
  transition: transform 0.15s ease;
}

.continue-card:hover {
  transform: translateY(-4px);
}

.continue-poster-wrap {
  position: relative;
  aspect-ratio: 2 / 3;
  border-radius: 8px;
  overflow: hidden;
  background: #1e2127;
}

/* Прогресс-бар внизу постера */
.progress-bar {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 4px;
  background: rgba(0, 0, 0, 0.6);
}
.progress-fill {
  height: 100%;
  background: #4a9eff;
  transition: width 0.3s ease;
}
</style>
