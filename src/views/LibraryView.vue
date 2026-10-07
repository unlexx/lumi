<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

import MediaGrid from '@/components/MediaGrid.vue'
import UndefinedCard from '@/components/UndefinedCard.vue'
import UndefinedModal from '@/components/UndefinedModal.vue'
import MovieCard from '@/components/MovieCard.vue'
import ShowCard from '@/components/ShowCard.vue'
import MovieModal from '@/components/MovieModal.vue'
import ShowModal from '@/components/ShowModal.vue'
import MatchModal from '@/components/MatchModal.vue'

import type {
    VideoFile,
    TvShow,
    TmdbSearchResult,
    MatchResult,
    UndefinedItem
} from '@/types'

import { useLibrary } from '@/composables/useLibrary'
import { usePosters } from '@/composables/usePosters'
import { usePlayer } from '@/composables/usePlayer'
import { useWatched } from '@/composables/useWatched'
import { useContinueWatching } from '@/composables/useContinueWatching'
import { useKeyboard } from '@/composables/useKeyboard'
import { useUndefined } from '@/composables/useUndefined'

const { library, loading, error, scanProgress, refresh } = useLibrary()
const { load: loadPosters } = usePosters(library)
const { play } = usePlayer()
const { continueWatching, load: loadContinueWatching } = useContinueWatching(library)
const { markMovie, markShow } = useWatched(loadContinueWatching)
const { items: undefinedItems, load: loadUndefined, removeMany: removeUndefinedMany } = useUndefined()

const selectedVideo = ref<VideoFile | null>(null)
const selectedShow = ref<TvShow | null>(null)
const selectedUndefined = ref<UndefinedItem | null>(null)

const matchTarget = ref<{
    media_type: 'movie' | 'tv_shows'
    title: string
    uids: string[]
} | null>(null)

useKeyboard({
    onEscape: () => {
        if (selectedVideo.value) selectedVideo.value = null
        else if (selectedShow.value) selectedShow.value = null
        else if (selectedUndefined.value) selectedUndefined.value = null
        else if (matchTarget.value) matchTarget.value = null
        else invoke('toggle_fullscreen').catch(console.error)
    }
})

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

function openMovie(video: VideoFile) {
    selectedVideo.value = video
}

function openShow(show: TvShow) {
    selectedShow.value = show
}

function openUndefined(item: UndefinedItem) {
    selectedUndefined.value = item
}

function playUndefined(path: string) {
    play(path)
    selectedUndefined.value = null
}

function openMatchModal(target: {
    media_type: 'movie' | 'tv_shows'
    title: string
    uids: string[]
}) {
    matchTarget.value = target
}

function openMatchModalForMovie(movie: VideoFile) {
    openMatchModal({
        media_type: 'movie',
        title: movie.tmdb?.title || movie.parsed.title,
        uids: [movie.uid]
    })
}

function openMatchModalForShow(show: TvShow) {
    const uids = show.seasons.flatMap((s) => s.episodes.map((ep) => ep.uid))
    if (!uids.length) {
        error.value = 'Нет эпизодов для сопоставления'
        return
    }
    openMatchModal({
        media_type: 'tv_shows',
        title: show.title,
        uids
    })
}

function matchUndefined(item: UndefinedItem) {
    openMatchModal({
        media_type: item.media_type,
        title: item.display_title,
        uids: [item.uid]
    })
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
            uids: target.uids
        })

        // Убираем любые совпадающие uid'ы из локального списка undefined
        removeUndefinedMany(target.uids)

        await refreshAll()
        matchTarget.value = null
    } catch (e) {
        console.error('Apply match error:', e)
        error.value = String(e)
    }
}
</script>

<template>
    <div>
        <div class="library-header">
            <button :disabled="loading" @click="refreshAll" title="Обновить библиотеку">↻</button>
        </div>

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
                    <article v-for="item in continueWatching" :key="item.path" class="continue-card"
                        @click="play(item.path, item.position)">
                        <div class="continue-poster-wrap">
                            <img v-if="item.poster_url" :src="item.poster_url" :alt="item.title" class="poster"
                                loading="lazy" />
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

            <section v-if="library.movies.length" class="section">
                <h2>Фильмы</h2>
                <div class="grid">
                    <MovieCard v-for="movie in library.movies" :key="movie.path" :movie="movie" @open="openMovie"
                        @play="play" @match="openMatchModalForMovie" @mark-watched="markMovie" />
                </div>
            </section>

            <section v-if="library.tv_shows.length" class="section">
                <h2>Сериалы</h2>
                <div class="grid">
                    <ShowCard v-for="show in library.tv_shows" :key="show.title" :show="show" @open="openShow"
                        @play="play" @match="openMatchModalForShow" @mark-watched="markShow" />
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

        <MovieModal v-if="selectedVideo" :movie="selectedVideo" @close="selectedVideo = null" @play="play" />

        <ShowModal v-if="selectedShow" :show="selectedShow" @close="selectedShow = null" @play="play" />

        <MatchModal v-if="matchTarget" :target="matchTarget" @close="matchTarget = null" @apply="applyMatch" />

        <UndefinedModal v-if="selectedUndefined" :item="selectedUndefined" @close="selectedUndefined = null"
            @play="playUndefined" @match="matchUndefined" />
    </div>
</template>

<style scoped>
.library-header {
    display: flex;
    justify-content: flex-end;
    margin-bottom: 0.5rem;
}

.library-header button {
    background: #2a2e35;
    color: #e6e6e6;
    border: 1px solid #3a3f47;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
}

.library-header button:hover:not(:disabled) {
    background: #353a42;
}

.library-header button:disabled {
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
    flex: 0 0 160px;
    /* фиксированная ширина карточки */
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

.poster {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    border-radius: 8px;
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