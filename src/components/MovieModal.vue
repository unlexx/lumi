<script setup lang="ts">
import type { VideoFile } from '@/types'

const props = defineProps<{
    movie: VideoFile
}>()

const emit = defineEmits<{
    (e: 'close'): void
    (e: 'play', path: string): void
}>()

function title(): string {
    return props.movie.tmdb?.title || props.movie.parsed.title
}
</script>

<template>
    <div class="modal-backdrop" @mousedown.self="emit('close')">
        <div class="modal">
            <button class="close" @click="emit('close')">×</button>
            <div class="modal-content">
                <img v-if="movie.tmdb?.poster_local" :src="movie.tmdb.poster_local" class="modal-poster" />
                <div class="modal-info">
                    <h2>
                        {{ title() }}
                        <span v-if="movie.parsed.year" class="year">({{ movie.parsed.year }})</span>
                    </h2>
                    <div v-if="movie.tmdb?.rating" class="modal-rating">
                        ★ {{ movie.tmdb.rating.toFixed(1) }}
                    </div>
                    <p v-if="movie.tmdb?.overview" class="overview">
                        {{ movie.tmdb.overview }}
                    </p>
                    <div class="tags">
                        <span v-if="movie.parsed.resolution">{{ movie.parsed.resolution }}</span>
                        <span v-if="movie.parsed.source">{{ movie.parsed.source }}</span>
                        <span v-if="movie.parsed.codec">{{ movie.parsed.codec }}</span>
                    </div>
                    <button class="play-btn" @click="emit('play', movie.path)">▶ Смотреть</button>
                    <div class="file-path">{{ movie.name }}</div>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
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
</style>