<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { onKeyStroke } from '@vueuse/core'
import Settings from './views/Settings.vue'
import { listen } from '@tauri-apps/api/event'
import MediaGrid from './components/MediaGrid.vue'
import UndefinedCard from './components/UndefinedCard.vue'
import UndefinedModal from './components/UndefinedModal.vue'
import { useUndefined } from './composables/useUndefined'
import type { UndefinedItem } from './composables/useUndefined'

const currentView = ref<'library' | 'settings'>('library')
const scanProgress = ref<{ current: number; total: number; folder: string } | null>(null)
// === Интерфейсы ===

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
  original_title: string | null
  overview: string | null
  poster_url: string | null
  poster_local: string | null
  rating: number | null
}

interface VideoFile {
  uid: string
  path: string
  name: string
  extension: string
  parsed: ParsedVideo
  tmdb: TmdbInfo | null
  media_type: 'movie' | 'tv_shows'
  watched: boolean
  position: number | null
  duration: number | null
}

interface Episode {
  number: number
  path: string
  name: string
  parsed: ParsedVideo
  watched: boolean
  position: number | null
  duration: number | null
}

interface Season {
  number: number
  episodes: Episode[]
}

interface TvShow {
  title: string
  year: number | null
  seasons: Season[]
  tmdb: TmdbInfo | null
}

interface Library {
  movies: VideoFile[]
  tv_shows: TvShow[]
}

interface ContinueItem {
  path: string
  title: string
  poster_url: string | null
  position: number
  duration: number
  progress: number
  media_type: 'movie' | 'tv_shows'
}

interface TmdbSearchResult {
  id: number
  title: string
  original_title: string | null
  overview: string | null
  poster_url: string | null
  poster_data: string | null
  year: number | null
}

interface MatchResult {
  tmdb_id: number
  title: string
  original_title: string | null
  overview: string | null
  poster_url: string | null
  rating: number | null
}

// === Состояние ===

const library = ref<Library>({ movies: [], tv_shows: [] })
const loading = ref(true)
const error = ref<string | null>(null)
const selectedVideo = ref<VideoFile | null>(null)
const selectedShow = ref<TvShow | null>(null)
const continueWatching = ref<ContinueItem[]>([])
const openMenuPath = ref<string | null>(null)
const matchModalOpen = ref(false)
const matchQuery = ref('')
const matchResults = ref<TmdbSearchResult[]>([])
const matchLoading = ref(false)
const matchTarget = ref<{
  path: string
  media_type: 'movie' | 'tv_shows'
  title: string
  uid?: string
} | null>(null)

// === Загрузка ===

async function refreshLibrary() {
  loading.value = true
  error.value = null
  scanProgress.value = null
  try {
    library.value = await invoke<Library>('scan_all')
    await loadPosters()
    await loadContinueWatching()
    await loadUndefined()
  } catch (e) {
    error.value = String(e)
    library.value = { movies: [], tv_shows: [] }
  } finally {
    loading.value = false
    scanProgress.value = null
  }
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
      // Эпизод?
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

    // Обновить секцию "Продолжить просмотр"
    await loadContinueWatching()
  })
  refreshLibrary()
})

async function loadPosters() {
  for (const movie of library.value.movies) {
    if (!movie.tmdb?.poster_url) continue
    try {
      const localPath = await invoke<string>('get_poster', {
        tmdbId: movie.tmdb.id,
        posterPath: extractPosterPath(movie.tmdb.poster_url)
      })
      movie.tmdb.poster_local = convertFileSrc(localPath)
    } catch (e) {
      console.error('Poster error for', movie.parsed.title, e)
    }
  }

  for (const show of library.value.tv_shows) {
    if (!show.tmdb?.poster_url) continue
    try {
      const localPath = await invoke<string>('get_poster', {
        tmdbId: show.tmdb.id,
        posterPath: extractPosterPath(show.tmdb.poster_url)
      })
      show.tmdb.poster_local = convertFileSrc(localPath)
    } catch (e) {
      console.error('Poster error for', show.title, e)
    }
  }
}

function extractPosterPath(url: string): string {
  const marker = '/t/p/w500'
  const idx = url.indexOf(marker)
  return idx >= 0 ? url.slice(idx + marker.length) : url
}

// === Действия ===

async function playVideo(path: string) {
  try {
    await invoke('play_video', { path })
  } catch (e) {
    console.error('Play error:', e)
    error.value = String(e)
  }
}

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

function isShowWatched(show: TvShow): boolean {
  const all = show.seasons.flatMap((s) => s.episodes)
  return all.length > 0 && all.every((e) => e.watched)
}

async function resumeVideo(item: ContinueItem) {
  try {
    await invoke('play_video', {
      path: item.path,
      startPosition: item.position
    })
  } catch (e) {
    console.error('Resume error:', e)
    error.value = String(e)
  }
}

async function loadContinueWatching() {
  // Собираем все пути из библиотеки
  const allPaths: string[] = [
    ...library.value.movies.map((m) => m.path),
    ...library.value.tv_shows.flatMap((s) =>
      s.seasons.flatMap((se) => se.episodes.map((e) => e.path))
    )
  ]

  const raw = await invoke<ContinueItem[]>('get_continue_watching', {
    paths: allPaths
  })

  // Обогащаем: постер, красивое название, media_type
  continueWatching.value = raw
    .map((item) => {
      // Фильм?
      const movie = library.value.movies.find((m) => m.path === item.path)
      if (movie) {
        return {
          ...item,
          title: movie.tmdb?.title || movie.parsed.title,
          poster_url: movie.tmdb?.poster_local || null,
          media_type: 'movie' as const
        }
      }

      // Эпизод?
      for (const show of library.value.tv_shows) {
        for (const season of show.seasons) {
          const ep = season.episodes.find((e) => e.path === item.path)
          if (ep) {
            return {
              ...item,
              title: `${show.title} — S${String(season.number).padStart(2, '0')}E${String(ep.number).padStart(2, '0')}`,
              poster_url: show.tmdb?.poster_local || null,
              media_type: 'tv_shows' as const
            }
          }
        }
      }

      return item
    })
    .filter((item) => item.poster_url || item.title)
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

function progressPercent(item: {
  position: number | null
  duration: number | null
}): number | null {
  if (!item.position || !item.duration || item.duration <= 0) return null
  const p = item.position / item.duration
  if (p <= 0.01) return null // едва начали — не показываем
  if (p >= 0.95) return null // досмотрели — тоже не показываем
  return p
}

function episodesWatched(show: TvShow): number {
  return show.seasons.flatMap((s) => s.episodes).filter((e) => e.watched).length
}

function episodesTotal(show: TvShow): number {
  return show.seasons.flatMap((s) => s.episodes).length
}

function allEpisodesWatched(show: TvShow): boolean {
  const total = episodesTotal(show)
  return total > 0 && episodesWatched(show) === total
}

function toggleMenu(path: string) {
  openMenuPath.value = openMenuPath.value === path ? null : path
}

// === Меню фильма ===

async function playMovieFromStart(movie: VideoFile) {
  await invoke('play_video', { path: movie.path, startPosition: null })
  openMenuPath.value = null
}

async function resumeMovie(movie: VideoFile) {
  if (movie.position == null) return
  await invoke('play_video', { path: movie.path, startPosition: movie.position })
  openMenuPath.value = null
}

async function markMovieWatched(movie: VideoFile, watched: boolean) {
  await invoke('set_watched_bulk', { paths: [movie.path], watched })
  movie.watched = watched
  if (!watched) {
    movie.position = 0
    movie.duration = 0
  }
  openMenuPath.value = null
  await loadContinueWatching()
}

// === Меню сериала ===

function firstEpisode(show: TvShow): Episode | null {
  for (const season of show.seasons) {
    if (season.episodes.length > 0) return season.episodes[0]
  }
  return null
}

function firstUnwatchedEpisode(show: TvShow): Episode | null {
  for (const season of show.seasons) {
    for (const ep of season.episodes) {
      if (!ep.watched) return ep
    }
  }
  return null
}

function episodeLabel(show: TvShow, ep: Episode): string {
  const season = show.seasons.find((s) => s.episodes.includes(ep))
  if (!season) return ''
  return `S${String(season.number).padStart(2, '0')}E${String(ep.number).padStart(2, '0')}`
}

async function playShowFromStart(show: TvShow) {
  const ep = firstEpisode(show)
  if (!ep) return
  await invoke('play_video', { path: ep.path, startPosition: null })
  openMenuPath.value = null
}

async function resumeShow(show: TvShow) {
  const ep = firstUnwatchedEpisode(show)
  if (!ep) return
  const startPosition = ep.position && ep.position > 0 ? ep.position : null
  await invoke('play_video', { path: ep.path, startPosition })
  openMenuPath.value = null
}

async function markShowWatched(show: TvShow, watched: boolean) {
  const paths = show.seasons.flatMap((s) => s.episodes.map((e) => e.path))
  if (paths.length === 0) return
  await invoke('set_watched_bulk', { paths, watched })
  for (const season of show.seasons) {
    for (const ep of season.episodes) {
      ep.watched = watched
      if (!watched) {
        ep.position = 0
        ep.duration = 0
      }
    }
  }
  openMenuPath.value = null
  await loadContinueWatching()
}

function openMatchModal(
  path: string,
  media_type: 'movie' | 'tv_shows',
  title: string,
  uid: string
) {
  matchTarget.value = { path, media_type, title, uid }
  matchQuery.value = title
  matchResults.value = []
  matchModalOpen.value = true
  openMenuPath.value = null
  // Автоматически искать при открытии
  searchMatch()
}

function closeMatchModal() {
  matchModalOpen.value = false
  matchTarget.value = null
  matchResults.value = []
}

async function searchMatch() {
  if (!matchTarget.value) return
  matchLoading.value = true
  try {
    const results = await invoke<TmdbSearchResult[]>('search_tmdb_manual', {
      query: matchQuery.value,
      mediaType: matchTarget.value.media_type
    })
    matchResults.value = results

    // Подгружаем постеры в фоне
    for (const result of matchResults.value) {
      if (!result.poster_url) continue
      result.poster_data = null
      invoke<string>('fetch_poster_preview', { url: result.poster_url })
        .then((data) => {
          result.poster_data = data
        })
        .catch((e) => console.error('Poster preview error:', e))
    }
  } catch (e) {
    console.error('Search error:', e)
  } finally {
    matchLoading.value = false
  }
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

    const uid = target.uid
    if (uid) {
      removeUndefined(uid)
    }

    await refreshLibrary()
    closeMatchModal()
  } catch (e) {
    console.error('Apply match error:', e)
    error.value = String(e)
  }
}

const { items: undefinedItems, load: loadUndefined, remove: removeUndefined } = useUndefined()
const selectedUndefined = ref<UndefinedItem | null>(null)

function openUndefined(item: UndefinedItem) {
  selectedUndefined.value = item
}

function closeUndefined() {
  selectedUndefined.value = null
}

function playUndefined(path: string) {
  playVideo(path)
  selectedUndefined.value = null
}

function matchUndefined(item: UndefinedItem) {
  matchTarget.value = {
    path: item.path,
    media_type: item.media_type,
    title: item.display_title,
    uid: item.uid
  }
  matchQuery.value = item.display_title
  selectedUndefined.value = null
  matchModalOpen.value = true
  searchMatch()
}

// === Клавиатура ===

onKeyStroke('Backspace', (e) => {
  if (currentView.value === 'settings') {
    e.preventDefault()
    currentView.value = 'library'
  }
})

onKeyStroke('F11', (e) => {
  e.preventDefault()
  invoke('toggle_fullscreen').catch(console.error)
})

onKeyStroke('Escape', () => {
  // Если открыта модалка — закрываем её, иначе выходим из полноэкранного
  if (selectedVideo.value) {
    selectedVideo.value = null
  } else if (selectedShow.value) {
    selectedShow.value = null
  } else if (openMenuPath.value) {
    openMenuPath.value = null
  } else {
    invoke('toggle_fullscreen').catch(console.error)
  }
})
</script>

<template>
  <main class="app" @click="openMenuPath = null">
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
          @click="refreshLibrary"
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
              @click="resumeVideo(item)"
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
            <article
              v-for="movie in library.movies"
              :key="movie.path"
              class="card"
              @click="openMovie(movie)"
            >
              <div class="poster-wrap">
                <img
                  v-if="movie.tmdb?.poster_local"
                  :src="movie.tmdb.poster_local"
                  :alt="movie.tmdb.title"
                  class="poster"
                  loading="lazy"
                />
                <div v-else class="poster placeholder">Нет постера</div>
                <div v-if="movie.tmdb?.rating" class="rating">
                  ★ {{ movie.tmdb.rating.toFixed(1) }}
                </div>
                <div v-if="movie.watched" class="watched-badge">✓</div>
                <div v-if="progressPercent(movie) !== null" class="progress-bar">
                  <div
                    class="progress-fill"
                    :style="{ width: progressPercent(movie)! * 100 + '%' }"
                  ></div>
                </div>
                <button class="menu-btn" @click.stop="toggleMenu(movie.path)">⋮</button>
                <div v-if="openMenuPath === movie.path" class="context-menu" @click.stop>
                  <button @click="playMovieFromStart(movie)">Смотреть с начала</button>
                  <button
                    v-if="movie.position && movie.position > 0 && !movie.watched"
                    @click="resumeMovie(movie)"
                  >
                    Продолжить с {{ formatTime(movie.position) }}
                  </button>
                  <button v-if="!movie.watched" @click="markMovieWatched(movie, true)">
                    Пометить просмотренным
                  </button>
                  <button v-else @click="markMovieWatched(movie, false)">Непросмотренно</button>
                  <button
                    @click="
                      openMatchModal(
                        movie.path,
                        'movie',
                        movie.tmdb?.title || movie.parsed.title,
                        movie.uid
                      )
                    "
                  >
                    Сопоставить
                  </button>
                </div>
              </div>
              <div class="card-title">
                {{ movie.tmdb?.title || movie.parsed.title }}
              </div>
              <div class="card-year">{{ movie.parsed.year || '' }}</div>
            </article>
          </div>
        </section>

        <!-- Сериалы -->
        <section v-if="library.tv_shows.length" class="section">
          <h2>Сериалы</h2>
          <div class="grid">
            <article
              v-for="show in library.tv_shows"
              :key="show.title"
              class="card"
              @click="openShow(show)"
            >
              <div class="poster-wrap">
                <img
                  v-if="show.tmdb?.poster_local"
                  :src="show.tmdb.poster_local"
                  :alt="show.title"
                  class="poster"
                  loading="lazy"
                />
                <div v-else class="poster placeholder">Сериал</div>
                <div v-if="show.tmdb?.rating" class="rating">
                  ★ {{ show.tmdb.rating.toFixed(1) }}
                </div>
                <div v-if="isShowWatched(show)" class="watched-badge">✓</div>
              </div>
              <button class="menu-btn" @click.stop="toggleMenu(show.title)">⋮</button>
              <div v-if="openMenuPath === show.title" class="context-menu" @click.stop>
                <button @click="playShowFromStart(show)">Смотреть с начала</button>
                <button v-if="firstUnwatchedEpisode(show)" @click="resumeShow(show)">
                  Продолжить с {{ episodeLabel(show, firstUnwatchedEpisode(show)!) }}
                </button>
                <button v-if="!allEpisodesWatched(show)" @click="markShowWatched(show, true)">
                  Пометить просмотренным
                </button>
                <button v-else @click="markShowWatched(show, false)">Непросмотренно</button>
                <button @click="openMatchModal('', 'tv_shows', show.title, '')">Сопоставить</button>
              </div>
              <div class="card-title">{{ show.title }}</div>
              <div class="card-year">
                <template v-if="show.year">{{ show.year }} · </template>
                <template v-if="allEpisodesWatched(show)">
                  <span class="all-watched">✓ Все просмотрено</span>
                </template>
                <template v-else>
                  <span v-if="episodesWatched(show) === 0"> {{ episodesTotal(show) }} сер. </span>
                  <span v-else>
                    Осталось {{ episodesTotal(show) - episodesWatched(show) }} из
                    {{ episodesTotal(show) }}
                  </span>
                </template>
              </div>
            </article>
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
    <div v-if="selectedVideo" class="modal-backdrop" @mousedown.self="closeMovie">
      <div class="modal">
        <button class="close" @click="closeMovie">×</button>
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
            <button class="play-btn" @click="playVideo(selectedVideo.path)">▶ Смотреть</button>
            <div class="file-path">{{ selectedVideo.name }}</div>
          </div>
        </div>
      </div>
    </div>

    <!-- Модалка сериала -->
    <div v-if="selectedShow" class="modal-backdrop" @mousedown.self="closeShow">
      <div class="modal">
        <button class="close" @click="closeShow">×</button>
        <h2>{{ selectedShow.title }}</h2>
        <div v-for="season in selectedShow.seasons" :key="season.number" class="season">
          <h3>Сезон {{ season.number }}</h3>
          <ul class="episode-list">
            <li
              v-for="ep in season.episodes"
              :key="ep.path"
              class="episode"
              :class="{ watched: ep.watched }"
            >
              <span class="ep-number">Серия {{ ep.number }}</span>
              <span class="ep-name">{{ ep.name }}</span>
              <span v-if="ep.watched" class="ep-watched">✓</span>
              <button class="ep-play" @click="playVideo(ep.path)">▶</button>
            </li>
          </ul>
        </div>
      </div>
    </div>
  </main>
  <div v-if="matchModalOpen" class="modal-backdrop" @mousedown.self="closeMatchModal">
    <div class="modal match-modal">
      <button class="close" @click="closeMatchModal">×</button>
      <h2>Сопоставить с TMDB</h2>

      <div class="search-row">
        <input
          v-model="matchQuery"
          type="text"
          placeholder="Название фильма или сериала"
          @keydown.enter="searchMatch"
        />
        <button @click="searchMatch" :disabled="matchLoading">
          {{ matchLoading ? 'Поиск...' : 'Искать' }}
        </button>
      </div>

      <div v-if="matchResults.length" class="results">
        <article
          v-for="result in matchResults"
          :key="result.id"
          class="result-item"
          @click="applyMatch(result)"
        >
          <img v-if="result.poster_data" :src="result.poster_data" class="result-poster" />
          <div v-else class="result-poster placeholder">
            {{ result.poster_url ? '…' : '—' }}
          </div>
          <div class="result-info">
            <div class="result-title">
              {{ result.title }}
              <span v-if="result.year" class="result-year">({{ result.year }})</span>
            </div>
            <div
              v-if="result.original_title && result.original_title !== result.title"
              class="result-original"
            >
              {{ result.original_title }}
            </div>
            <div v-if="result.overview" class="result-overview">
              {{ result.overview }}
            </div>
          </div>
        </article>
      </div>
      <p v-else-if="!matchLoading && matchQuery" class="no-results">
        Ничего не найдено. Попробуйте другое название.
      </p>
    </div>
  </div>
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
  align-items: flex-start;
}

.modal-poster {
  width: 240px;
  height: auto;
  border-radius: 8px;
  flex-shrink: 0;
  object-fit: contain;
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
  font-size: 1.2rem;
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

.season {
  margin-top: 1.5rem;
}

.season h3 {
  margin: 0 0 0.75rem;
  font-size: 1rem;
  color: #aaa;
  font-weight: 500;
}

.episode-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.episode {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.5rem 0;
  border-bottom: 1px solid #2a2e35;
}

.ep-number {
  font-weight: 600;
  min-width: 80px;
  color: #4a9eff;
}

.ep-name {
  flex: 1;
  font-size: 0.85rem;
  color: #888;
  font-family: monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ep-play {
  background: #2a2e35;
  color: #e6e6e6;
  border: 1px solid #3a3f47;
  padding: 0.3rem 0.75rem;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.9rem;
}

.ep-play:hover {
  background: #353a42;
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
.ep-watched {
  color: #4a9eff;
  font-weight: bold;
  font-size: 1rem;
  margin-right: 0.5rem;
}

.episode.watched .ep-number {
  color: #4a9eff;
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

.all-watched {
  color: #4a9eff;
  font-weight: 500;
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

.card:hover .menu-btn,
.continue-card:hover .menu-btn {
  opacity: 1;
}

.menu-btn:hover {
  background: rgba(0, 0, 0, 0.9);
}

/* Рейтинг сдвигаем левее, чтобы не конфликтовал с «⋮» */
.rating {
  right: 6px;
}

.context-menu {
  position: absolute;
  bottom: 38px; /* было top: 38px */
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

.match-modal {
  max-width: 700px;
}

.search-row {
  display: flex;
  gap: 0.5rem;
  margin: 1rem 0;
}

.search-row input {
  flex: 1;
  background: #2a2e35;
  border: 1px solid #3a3f47;
  color: #e6e6e6;
  padding: 0.5rem 0.75rem;
  border-radius: 6px;
  font-size: 0.9rem;
}

.search-row button {
  background: #4a9eff;
  color: #fff;
  border: none;
  padding: 0.5rem 1.25rem;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.9rem;
}

.search-row button:hover:not(:disabled) {
  background: #3a8eef;
}

.search-row button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.results {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  max-height: 60vh;
  overflow-y: auto;
}

.result-item {
  display: flex;
  gap: 1rem;
  padding: 0.75rem;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.15s ease;
}

.result-item:hover {
  background: #2a2e35;
}

.result-poster {
  width: 60px;
  height: 90px;
  object-fit: cover;
  border-radius: 4px;
  flex-shrink: 0;
}

.result-poster.placeholder {
  background: #2a2e35;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #666;
  font-size: 0.75rem;
}

.result-info {
  flex: 1;
  min-width: 0;
}

.result-title {
  font-weight: 600;
  font-size: 0.95rem;
}

.result-year {
  color: #888;
  font-weight: 400;
}

.result-original {
  color: #888;
  font-size: 0.8rem;
  margin-top: 0.15rem;
}

.result-overview {
  color: #aaa;
  font-size: 0.8rem;
  margin-top: 0.35rem;
  display: -webkit-box;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.no-results {
  color: #888;
  text-align: center;
  padding: 2rem 0;
}

.result-poster.placeholder {
  background: #2a2e35;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #666;
  font-size: 0.9rem;
}
</style>
