<script setup lang="ts">
import { ref } from 'vue'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { onMounted } from 'vue'
import { onKeyStroke } from '@vueuse/core'
import Settings from './views/Settings.vue'

const currentView = ref<'library' | 'settings'>('library')

interface ParsedVideo {
  title: string
  year: number | null
  resolution: string | null
  source: string | null
  codec: string | null
}

interface TmdbInfo {
  id: number
  title: string
  overview: string | null
  poster_url: string | null
  poster_local: string | null
  rating: number | null
}

interface VideoFile {
  path: string
  name: string
  extension: string
  parsed: ParsedVideo
  tmdb: TmdbInfo | null
}

const videos = ref<VideoFile[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const selectedVideo = ref<VideoFile | null>(null)

async function refreshLibrary() {
  loading.value = true
  error.value = null
  try {
    videos.value = await invoke<VideoFile[]>('scan_all')
    loadPosters()
  } catch (e) {
    error.value = String(e)
    videos.value = []
  } finally {
    loading.value = false
  }
}

onMounted(refreshLibrary)

async function loadPosters() {
  for (const video of videos.value) {
    if (!video.tmdb?.poster_url) continue
    try {
      const localPath = await invoke<string>('get_poster', {
        tmdbId: video.tmdb.id,
        posterPath: extractPosterPath(video.tmdb.poster_url)
      })
      video.tmdb.poster_local = convertFileSrc(localPath)
    } catch (e) {
      console.error('Poster error for', video.parsed.title, e)
    }
  }
}

function extractPosterPath(url: string): string {
  const marker = '/t/p/w500'
  const idx = url.indexOf(marker)
  return idx >= 0 ? url.slice(idx + marker.length) : url
}

async function playVideo(video: VideoFile) {
  try {
    await invoke('play_video', { path: video.path })
  } catch (e) {
    console.error('Play error:', e)
    error.value = String(e)
  }
}

function openDetails(video: VideoFile) {
  selectedVideo.value = video
}

function closeDetails() {
  selectedVideo.value = null
}

onKeyStroke('Backspace', (e) => {
  if (currentView.value === 'settings') {
    e.preventDefault()
    currentView.value = 'library'
  }
})

onKeyStroke('Escape', () => {
  if (selectedVideo.value) selectedVideo.value = null
})
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
      </nav>
    </header>
    <Settings v-if="currentView === 'settings'" />
    <div v-else>
      <p v-if="error" class="error">{{ error }}</p>
      <section v-if="videos.length" class="grid">
        <article v-for="video in videos" :key="video.path" class="card" @click="openDetails(video)">
          <div class="poster-wrap">
            <img
              v-if="video.tmdb?.poster_local"
              :src="video.tmdb.poster_local"
              :alt="video.tmdb.title"
              class="poster"
              loading="lazy"
            />
            <div v-else class="poster placeholder">Нет постера</div>
            <div v-if="video.tmdb?.rating" class="rating">★ {{ video.tmdb.rating.toFixed(1) }}</div>
          </div>
          <div class="card-title">{{ video.tmdb?.title || video.parsed.title }}</div>
          <div class="card-year">{{ video.parsed.year || '' }}</div>
        </article>
      </section>
    </div>
    <!-- Модалка с деталями -->
    <div v-if="selectedVideo" class="modal-backdrop" @click="closeDetails">
      <div class="modal" @click.stop>
        <button class="close" @click="closeDetails">×</button>
        <div class="modal-content">
          <img
            v-if="selectedVideo.tmdb?.poster_local"
            :src="selectedVideo.tmdb.poster_local"
            class="modal-poster"
          />
          <div class="modal-info">
            <h2>
              {{ selectedVideo.tmdb?.title || selectedVideo.parsed.title }}
              <span v-if="selectedVideo.parsed.year" class="year">
                ({{ selectedVideo.parsed.year }})
              </span>
            </h2>
            <div v-if="selectedVideo.tmdb?.rating" class="modal-rating">
              ★ {{ selectedVideo.tmdb.rating.toFixed(1) }}
            </div>
            <p v-if="selectedVideo.tmdb?.overview" class="overview">
              {{ selectedVideo.tmdb.overview }}
            </p>
            <div class="tags">
              <span v-if="selectedVideo.parsed.resolution">{{
                selectedVideo.parsed.resolution
              }}</span>
              <span v-if="selectedVideo.parsed.source">{{ selectedVideo.parsed.source }}</span>
              <span v-if="selectedVideo.parsed.codec">{{ selectedVideo.parsed.codec }}</span>
            </div>
            <button class="play-btn" @click="playVideo(selectedVideo)">▶ Смотреть</button>
            <div class="file-path">{{ selectedVideo.name }}</div>
          </div>
        </div>
      </div>
    </div>
  </main>
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

.card {
  cursor: pointer;
  transition: transform 0.15s ease;
}

.card:hover {
  transform: translateY(-4px);
}

.poster-wrap {
  position: relative;
  aspect-ratio: 2 / 3;
  border-radius: 8px;
  overflow: hidden;
  background: #1e2127;
}

.poster {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
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

/* Модалка */
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
  max-width: 800px;
  width: 100%;
  max-height: 90vh;
  overflow-y: auto;
  position: relative;
  padding: 1.5rem;
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

.modal-content {
  display: flex;
  gap: 1.5rem;
}

.modal-poster {
  width: 240px;
  border-radius: 8px;
  flex-shrink: 0;
}

.modal-info {
  flex: 1;
}

.modal-info h2 {
  margin: 0 0 0.5rem 0;
  font-size: 1.4rem;
}

.year {
  color: #888;
  font-weight: 400;
}

.modal-rating {
  color: #ffd166;
  margin-bottom: 0.75rem;
}

.overview {
  color: #bbb;
  line-height: 1.5;
  margin: 0.75rem 0;
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

.play-btn {
  background: #4a9eff;
  color: #fff;
  border: none;
  padding: 0.6rem 1.5rem;
  border-radius: 6px;
  font-size: 1rem;
  cursor: pointer;
  margin-top: 0.5rem;
}

.play-btn:hover {
  background: #3a8eef;
}

.file-path {
  margin-top: 1rem;
  font-size: 0.75rem;
  color: #666;
  font-family: monospace;
  word-break: break-all;
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
</style>
