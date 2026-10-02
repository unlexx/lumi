<script setup lang="ts">
import type { TvShow } from '@/types'

defineProps<{
    show: TvShow
}>()

const emit = defineEmits<{
    (e: 'close'): void
    (e: 'play', path: string): void
}>()
</script>

<template>
    <div class="modal-backdrop" @mousedown.self="emit('close')">
        <div class="modal">
            <button class="close" @click="emit('close')">×</button>
            <h2>{{ show.title }}</h2>
            <div v-for="season in show.seasons" :key="season.number" class="season">
                <h3>Сезон {{ season.number }}</h3>
                <ul class="episode-list">
                    <li v-for="ep in season.episodes" :key="ep.path" class="episode" :class="{ watched: ep.watched }">
                        <span class="ep-number">Серия {{ ep.number }}</span>
                        <span class="ep-name">{{ ep.name }}</span>
                        <span v-if="ep.watched" class="ep-watched">✓</span>
                        <button class="ep-play" @click="emit('play', ep.path)">▶</button>
                    </li>
                </ul>
            </div>
        </div>
    </div>
</template>

<style scoped>
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

.ep-watched {
    color: #4a9eff;
    font-weight: bold;
    font-size: 1rem;
    margin-right: 0.5rem;
}

.episode.watched .ep-number {
    color: #4a9eff;
}
</style>