<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { onKeyStroke } from '@vueuse/core'
import type { VideoFile, TvShow, Episode } from '@/types'
import { usePlayer } from '@/composables/usePlayer'

const { play } = usePlayer()
const props = defineProps<{
    item: VideoFile | TvShow
}>()

const emit = defineEmits<{
    (e: 'close'): void
}>()

onKeyStroke('Escape', (e) => {
    e.preventDefault()
    emit('close')
})

function isShow(x: VideoFile | TvShow): x is TvShow {
    return 'seasons' in x
}

const isMovie = computed(() => !isShow(props.item))
const movie = computed<VideoFile | null>(() => (isMovie.value ? (props.item as VideoFile) : null))
const show = computed<TvShow | null>(() => (isShow(props.item) ? props.item : null))

const title = computed(() => {
    if (movie.value) return movie.value.tmdb?.title || movie.value.parsed.title
    return show.value?.title ?? ''
})

const year = computed(() => {
    if (movie.value) return movie.value.parsed.year
    return show.value?.year ?? null
})

const posterUrl = computed(() => {
    if (movie.value) return movie.value.tmdb?.poster_local || null
    return show.value?.tmdb?.poster_local || null
})

const rating = computed(() => {
    if (movie.value) return movie.value.tmdb?.rating ?? null
    return show.value?.tmdb?.rating ?? null
})

const overview = computed(() => {
    if (movie.value) return movie.value.tmdb?.overview ?? null
    return show.value?.tmdb?.overview ?? null
})

const resumeMoviePosition = computed(() => {
    if (!movie.value) return null
    const m = movie.value
    if (m.watched) return null
    if (!m.position || m.position <= 0) return null
    if (m.duration && m.position / m.duration >= 0.95) return null
    return m.position
})

// === LUMI-24: trailer ===

type TrailerState = 'loading' | 'available' | 'hidden'

const trailerState = ref<TrailerState>('loading')
const trailerKey = ref<string | null>(null)
const trailerThumb = ref<string | null>(null)

const mediaTypeForTmdb = computed(() =>
    movie.value ? 'movie' : 'tv_shows'
)

const tmdbIdForTrailer = computed<number | null>(() => {
    if (movie.value) return movie.value.tmdb?.id ?? null
    return show.value?.tmdb?.id ?? null
})

const uidForTrailer = computed<string | null>(() => {
    if (movie.value) return movie.value.uid
    // для сериала берём первый эпизод — трейлер шоу-level,
    // но uid нужен как ключ в media_items; любая серия шоу сойдёт
    const first = show.value?.seasons[0]?.episodes[0]
    return first?.uid ?? null
})

async function openTrailer() {
    if (!trailerKey.value) return
    try {
        await openUrl(`https://youtu.be/${trailerKey.value}`)
    } catch (e) {
        console.error('[DetailView] openUrl failed:', e)
    }
}

async function loadTrailer() {
    const tmdbId = tmdbIdForTrailer.value
    const uid = uidForTrailer.value
    if (!tmdbId || !uid) {
        trailerState.value = 'hidden'
        return
    }

    trailerState.value = 'loading'
    try {
        const key = await invoke<string | null>('fetch_trailer', {
            tmdbId,
            mediaType: mediaTypeForTmdb.value,
            uid
        })

        if (!key) {
            trailerState.value = 'hidden'
            return
        }

        trailerKey.value = key
        trailerState.value = 'available'

        // Thumbnail — best-effort, не блокирует кнопку
        try {
            const path = await invoke<string>('get_trailer_thumbnail', {
                tmdbId,
                mediaType: mediaTypeForTmdb.value,
                uid
            })
            trailerThumb.value = convertFileSrc(path)
        } catch (e) {
            console.warn('[DetailView] trailer thumbnail failed:', e)
        }
    } catch (e) {
        console.error('[DetailView] fetch_trailer failed:', e)
        // Ошибку трактуем как «трейлера нет» — на следующем заходе попробуем ещё раз
        trailerState.value = 'hidden'
    }
}

function episodesWatched(show: TvShow): number {
    return show.seasons.flatMap((s) => s.episodes).filter((e) => e.watched).length
}

function episodesTotal(show: TvShow): number {
    return show.seasons.flatMap((s) => s.episodes).length
}

const firstUnwatchedEpisode = computed<Episode | null>(() => {
    if (!show.value) return null
    for (const season of show.value.seasons) {
        for (const ep of season.episodes) {
            if (!ep.watched) return ep
        }
    }
    return null
})

function episodeLabel(ep: Episode): string {
    if (!show.value) return ''
    const season = show.value.seasons.find((s) => s.episodes.includes(ep))
    if (!season) return ''
    return `S${String(season.number).padStart(2, '0')}E${String(ep.number).padStart(2, '0')}`
}

function playMovie() {
    if (!movie.value) return
    play(movie.value.path, null)
}

function resumeMovie() {
    if (!movie.value || resumeMoviePosition.value === null) return
    play(movie.value.path, resumeMoviePosition.value)
}

function playEpisode(ep: Episode) {
    const start = ep.position && ep.position > 0 ? ep.position : null
    play(ep.path, start)
}

// === LUMI-18b: episode stills ===

const stills = ref<Record<string, string>>({})

function stillUrl(ep: Episode): string | null {
    if (stills.value[ep.uid]) return stills.value[ep.uid]
    if (ep.episode_still_local) return convertFileSrc(ep.episode_still_local)
    return null
}

async function mapLimit<T, R>(
    items: T[],
    limit: number,
    fn: (item: T) => Promise<R>
): Promise<R[]> {
    const results: R[] = new Array(items.length)
    let idx = 0
    const workers = Array.from(
        { length: Math.min(limit, items.length) },
        async () => {
            while (idx < items.length) {
                const i = idx++
                results[i] = await fn(items[i])
            }
        }
    )
    await Promise.all(workers)
    return results
}

onMounted(async () => {
    await loadTrailer()

    if (!show.value || !show.value.tmdb) return

    const tmdbId = show.value.tmdb.id
    const all: { ep: Episode; season: number }[] = []
    for (const s of show.value.seasons) {
        for (const ep of s.episodes) {
            all.push({ ep, season: s.number })
        }
    }

    // Этап 1: fetch meta
    const needMeta = all.filter((x) => !x.ep.episode_meta_fetched)
    if (needMeta.length) {
        console.log(`[DetailView] fetching meta for ${needMeta.length} episodes`)
        await mapLimit(needMeta, 4, async (x) => {
            try {
                const meta = await invoke<{
                    name: string | null
                    overview: string | null
                    still_url: string | null
                    still_path: string | null
                }>('fetch_episode_meta', {
                    tmdbId,
                    season: x.season,
                    episode: x.ep.number,
                    uid: x.ep.uid
                })
                x.ep.episode_name = meta.name
                x.ep.episode_overview = meta.overview
                x.ep.episode_still_path = meta.still_path
                x.ep.episode_meta_fetched = true
            } catch (e) {
                console.error(`[DetailView] fetch_episode_meta failed for ${x.ep.uid}:`, e)
            }
        })
    }

    // Этап 2: fetch stills
    const needStill = all.filter(
        (x) => x.ep.episode_still_path && !x.ep.episode_still_local
    )
    if (needStill.length) {
        console.log(`[DetailView] fetching stills for ${needStill.length} episodes`)
        await mapLimit(needStill, 4, async (x) => {
            try {
                const path = await invoke<string>('get_episode_still', {
                    tmdbId,
                    season: x.season,
                    episode: x.ep.number,
                    stillPath: x.ep.episode_still_path!
                })
                x.ep.episode_still_local = path
                stills.value[x.ep.uid] = convertFileSrc(path)
            } catch (e) {
                console.error(`[DetailView] get_episode_still failed for ${x.ep.uid}:`, e)
            }
        })
    }
})
</script>

<template>
    <div class="detail">
        <div class="detail-hero">
            <div class="hero-backdrop">
                <img v-if="posterUrl" :src="posterUrl" :alt="title" class="hero-image" />
            </div>

            <div class="hero-content">
                <div class="hero-poster">
                    <img v-if="posterUrl" :src="posterUrl" :alt="title" />
                    <div v-else class="poster-placeholder">—</div>
                </div>

                <div class="hero-info">
                    <h1>
                        {{ title }}
                        <span v-if="year" class="hero-year">({{ year }})</span>
                    </h1>

                    <div v-if="rating" class="hero-rating">
                        ★ {{ rating.toFixed(1) }}
                    </div>

                    <div v-if="show" class="hero-progress">
                        <template v-if="episodesWatched(show) === episodesTotal(show)">
                            ✓ Все просмотрено
                        </template>
                        <template v-else-if="episodesWatched(show) > 0">
                            Осталось {{ episodesTotal(show) - episodesWatched(show) }} из
                            {{ episodesTotal(show) }} серий
                        </template>
                        <template v-else>
                            {{ episodesTotal(show) }} серий
                        </template>
                    </div>

                    <div v-if="overview" class="hero-overview">
                        {{ overview }}
                    </div>

                    <div v-if="movie" class="hero-tags">
                        <span v-if="movie.parsed.resolution">{{ movie.parsed.resolution }}</span>
                        <span v-if="movie.parsed.source">{{ movie.parsed.source }}</span>
                        <span v-if="movie.parsed.codec">{{ movie.parsed.codec }}</span>
                    </div>

                    <div class="hero-actions">
                        <button v-if="isMovie" class="btn-primary" @click="playMovie">
                            ▶ Смотреть
                        </button>
                        <button v-if="isMovie && resumeMoviePosition" class="btn-secondary" @click="resumeMovie">
                            ↻ Продолжить
                        </button>

                        <button v-if="show && firstUnwatchedEpisode" class="btn-primary"
                            @click="playEpisode(firstUnwatchedEpisode)">
                            ▶ Смотреть {{ episodeLabel(firstUnwatchedEpisode) }}
                        </button>
                        <button v-if="trailerState !== 'hidden'" class="btn-trailer"
                            :disabled="trailerState === 'loading'" @click="openTrailer">
                            <span v-if="trailerState === 'loading'" class="trailer-spinner" />
                            <img v-else-if="trailerThumb" :src="trailerThumb" alt="" class="trailer-thumb" />
                            <span v-else class="trailer-play-icon">▶</span>
                            <span>{{ trailerState === 'loading' ? 'Загрузка…' : 'Трейлер' }}</span>
                        </button>
                    </div>

                    <div v-if="movie" class="hero-path">{{ movie.name }}</div>
                </div>
            </div>
        </div>

        <div v-if="show" class="detail-body">
            <div v-for="season in show.seasons" :key="season.number" class="season">
                <h2>Сезон {{ season.number }}</h2>
                <ul class="episode-list">
                    <li v-for="ep in season.episodes" :key="ep.uid" class="episode" :class="{ watched: ep.watched }">
                        <div class="ep-thumb">
                            <img v-if="stillUrl(ep)" :src="stillUrl(ep)!" :alt="ep.episode_name || ep.name"
                                loading="lazy" />
                            <div v-else class="ep-thumb-placeholder">▶</div>
                        </div>
                        <div class="ep-info">
                            <div class="ep-number">{{ episodeLabel(ep) }}</div>
                            <div class="ep-name">{{ ep.episode_name || ep.name }}</div>
                        </div>
                        <span v-if="ep.watched" class="ep-watched">✓</span>
                        <button class="ep-play" @click="playEpisode(ep)">▶</button>
                    </li>
                </ul>
            </div>
        </div>

        <button class="detail-close" @click="emit('close')" title="Назад (Esc)">← Назад</button>
    </div>
</template>

<style scoped>
/* ... все стили как были + новые для .ep-thumb / .ep-info / .ep-thumb-placeholder ... */

.detail {
    position: relative;
    min-height: 100vh;
    background: #14161a;
    padding-bottom: 4rem;
}

.detail-hero {
    position: relative;
    padding: 2rem 4rem 0;
    margin-bottom: 2rem;
}

.hero-backdrop {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 420px;
    overflow: hidden;
    pointer-events: none;
}

.hero-image {
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: blur(60px) brightness(0.5);
    transform: scale(1.15);
}

.hero-content {
    position: relative;
    z-index: 2;
    display: flex;
    gap: 2rem;
    align-items: flex-start;
    padding-top: 3rem;
    max-width: 1400px;
    margin: 0 auto;
}

.hero-poster {
    flex: 0 0 240px;
    aspect-ratio: 2 / 3;
    border-radius: 12px;
    overflow: hidden;
    background: #1e2127;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.6);
}

.hero-poster img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
}

.poster-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #666;
    font-size: 2rem;
}

.hero-info {
    flex: 1;
    padding-top: 2rem;
    max-width: 800px;
}

.hero-info h1 {
    margin: 0 0 0.5rem 0;
    font-size: 2rem;
    font-weight: 600;
    line-height: 1.15;
}

.hero-year {
    color: #888;
    font-weight: 400;
    font-size: 1.5rem;
}

.hero-rating {
    color: #ffd166;
    font-size: 1.1rem;
    margin-bottom: 1rem;
}

.hero-progress {
    color: #aaa;
    font-size: 0.95rem;
    margin-bottom: 1rem;
}

.hero-overview {
    color: #ccc;
    line-height: 1.5;
    font-size: 1rem;
    max-width: 720px;
    margin-bottom: 1rem;
}

.hero-tags {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    margin-bottom: 1.5rem;
}

.hero-tags span {
    background: #2a2e35;
    padding: 0.25rem 0.6rem;
    border-radius: 4px;
    font-size: 0.8rem;
    color: #aaa;
}

.hero-actions {
    display: flex;
    gap: 0.75rem;
    margin-bottom: 1rem;
}

.btn-primary,
.btn-secondary {
    border: none;
    padding: 0.7rem 1.5rem;
    border-radius: 6px;
    font-size: 1rem;
    cursor: pointer;
    transition: background 0.15s ease;
}

.btn-primary {
    background: #4a9eff;
    color: #fff;
}

.btn-primary:hover {
    background: #3a8eef;
}

.btn-secondary {
    background: #2a2e35;
    color: #e6e6e6;
    border: 1px solid #3a3f47;
}

.btn-secondary:hover {
    background: #353a42;
}

.hero-path {
    font-size: 0.75rem;
    color: #666;
    font-family: monospace;
    word-break: break-all;
    max-width: 720px;
}

.detail-body {
    max-width: 1400px;
    margin: 2rem auto 0;
    padding: 0 4rem;
}

.season {
    margin-bottom: 2rem;
}

.season h2 {
    margin: 0 0 1rem;
    font-size: 1.2rem;
    color: #ccc;
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
    padding: 0.6rem 0.75rem;
    border-radius: 6px;
    border-bottom: 1px solid #2a2e35;
    transition: background 0.15s ease;
}

.episode:hover {
    background: #1e2127;
}

.ep-thumb {
    flex: 0 0 120px;
    aspect-ratio: 16 / 9;
    border-radius: 6px;
    overflow: hidden;
    background: #1e2127;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #4a4f58;
    font-size: 1.2rem;
}

.ep-thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
}

.ep-info {
    flex: 1;
    min-width: 0;
}

.ep-number {
    font-weight: 600;
    color: #4a9eff;
    font-family: monospace;
    font-size: 0.85rem;
}

.ep-name {
    font-size: 0.9rem;
    color: #ccc;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-top: 0.15rem;
}

.ep-play {
    background: #2a2e35;
    color: #e6e6e6;
    border: 1px solid #3a3f47;
    padding: 0.35rem 0.85rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9rem;
}

.ep-play:hover {
    background: #353a42;
}

.ep-watched {
    color: #4a9eff;
    font-weight: bold;
    font-size: 1rem;
}

.episode.watched .ep-number {
    color: #4a9eff;
}

.detail-close {
    position: fixed;
    top: 1.5rem;
    left: 1.5rem;
    background: rgba(30, 33, 39, 0.9);
    color: #e6e6e6;
    border: 1px solid #3a3f47;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
    z-index: 100;
    backdrop-filter: blur(8px);
}

.detail-close:hover {
    background: #2a2e35;
}

.hero-backdrop::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(to bottom,
            rgba(20, 22, 26, 0.3) 0%,
            rgba(20, 22, 26, 0.7) 50%,
            #14161a 100%);
}

.btn-trailer {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    background: #2a2e35;
    color: #e6e6e6;
    border: 1px solid #3a3f47;
    padding: 0.4rem 1rem 0.4rem 0.4rem;
    border-radius: 6px;
    font-size: 1rem;
    cursor: pointer;
    transition: background 0.15s ease;
}

.btn-trailer:hover:not(:disabled) {
    background: #353a42;
}

.btn-trailer:disabled {
    cursor: default;
    opacity: 0.75;
}

.trailer-thumb {
    width: 96px;
    height: 54px;
    object-fit: cover;
    border-radius: 4px;
    display: block;
}

.trailer-play-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 96px;
    height: 54px;
    border-radius: 4px;
    background: #1e2127;
    color: #4a9eff;
}

.trailer-spinner {
    display: inline-block;
    width: 16px;
    height: 16px;
    margin-left: 0.6rem;
    border: 2px solid #3a3f47;
    border-top-color: #4a9eff;
    border-radius: 50%;
    animation: trailer-spin 0.8s linear infinite;
}

@keyframes trailer-spin {
    to {
        transform: rotate(360deg);
    }
}
</style>