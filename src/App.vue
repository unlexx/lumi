<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { convertFileSrc } from '@tauri-apps/api/core'

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

async function selectFolder() {
  const folder = await open({
    directory: true,
    multiple: false,
    title: 'Выберите папку с видео'
  })

  if (!folder) return

  loading.value = true
  error.value = null

  try {
    videos.value = await invoke<VideoFile[]>('scan_videos', {
      folderPath: folder
    })
    loadPosters()
  } catch (e) {
    error.value = String(e)
    videos.value = []
  } finally {
    loading.value = false
  }
}
async function loadPosters() {
  for (const video of videos.value) {
    if (!video.tmdb?.poster_url) continue
    try {
      // Из poster_url вытаскиваем tmdb_id и путь
      const localPath = await invoke<string>('get_poster', {
        tmdbId: video.tmdb.id,
        posterPath: extractPosterPath(video.tmdb.poster_url)
      })
      video.tmdb.poster_local = convertFileSrc(localPath)
    } catch (e) {
      console.error('Poster error for', video.parsed.title, e)
    }
  }

  // Утилита: "https://image.tmdb.org/t/p/w500/abc.jpg" → "/abc.jpg"
  function extractPosterPath(url: string): string {
    const marker = '/t/p/w500'
    const idx = url.indexOf(marker)
    return idx >= 0 ? url.slice(idx + marker.length) : url
  }
}
</script>

<template>
  <main class="container">
    <h1>Lumi</h1>
    <button @click="selectFolder" :disabled="loading">
      {{ loading ? 'Сканирование...' : 'Выбрать папку' }}
    </button>
    <p v-if="error" class="error">{{ error }}</p>
    <ul v-if="videos.length" class="video-list">
      <li v-for="video in videos" :key="video.path" class="video-item">
        <div class="poster-wrap">
          <img
            v-if="video.tmdb?.poster_local"
            :src="video.tmdb.poster_local"
            :alt="video.tmdb.title"
            class="poster"
          />
          <div v-else-if="video.tmdb?.poster_url" class="poster placeholder">Загрузка...</div>
        </div>
        <div class="info">
          <div class="title">
            {{ video.tmdb?.title || video.parsed.title }}
            <span v-if="video.parsed.year" class="year">({{ video.parsed.year }})</span>
          </div>
          <div v-if="video.tmdb?.rating" class="rating">★ {{ video.tmdb.rating.toFixed(1) }}</div>
          <div v-if="video.tmdb?.overview" class="overview">
            {{ video.tmdb.overview }}
          </div>
        </div>
      </li>
    </ul>
  </main>
</template>

<style scoped>
.container {
  padding: 2rem;
  font-family: sans-serif;
}
.error {
  color: red;
}
.video-list {
  list-style: none;
  padding: 0;
}
.video-list li {
  padding: 1rem 0;
  border-bottom: 1px solid #eee;
}
.title {
  font-size: 1.1rem;
  font-weight: 600;
}
.year {
  color: #888;
  font-weight: 400;
}
.meta {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.25rem;
  color: #666;
  font-size: 0.85rem;
}
.raw {
  margin-top: 0.25rem;
  color: #aaa;
  font-size: 0.75rem;
  font-family: monospace;
}
.video-item {
  display: flex;
  gap: 1rem;
  padding: 1rem 0;
  border-bottom: 1px solid #eee;
}
.poster-wrap {
  flex: 0 0 120px;
}
.poster {
  width: 120px;
  border-radius: 4px;
  display: block;
}
.info {
  flex: 1;
}
</style>
