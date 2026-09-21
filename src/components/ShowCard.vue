<script setup lang="ts">
import { computed } from 'vue'
import type { TvShow, Episode } from '@/types'
import MediaPoster from './MediaPoster.vue'

const props = defineProps<{
  show: TvShow
  menuOpen: boolean
}>()

const emit = defineEmits<{
  (e: 'open', show: TvShow): void
  (e: 'play', path: string, startPosition: number | null): void
  (e: 'match', show: TvShow): void
  (e: 'mark-watched', show: TvShow, watched: boolean): void
  (e: 'toggle-menu', key: string): void
}>()

const menuKey = computed(() => props.show.title)

function episodesWatched(): number {
  return props.show.seasons.flatMap((s) => s.episodes).filter((e) => e.watched).length
}

function episodesTotal(): number {
  return props.show.seasons.flatMap((s) => s.episodes).length
}

function allWatched(): boolean {
  const total = episodesTotal()
  return total > 0 && episodesWatched() === total
}

function showProgress(): number | null {
  const total = episodesTotal()
  if (total === 0) return null
  const watched = episodesWatched()
  if (watched === 0) return null
  if (watched === total) return null
  return watched / total
}

function firstEpisode(): Episode | null {
  for (const season of props.show.seasons) {
    if (season.episodes.length > 0) return season.episodes[0]
  }
  return null
}

function firstUnwatchedEpisode(): Episode | null {
  for (const season of props.show.seasons) {
    for (const ep of season.episodes) {
      if (!ep.watched) return ep
    }
  }
  return null
}

function episodeLabel(ep: Episode): string {
  const season = props.show.seasons.find((s) => s.episodes.includes(ep))
  if (!season) return ''
  return `S${String(season.number).padStart(2, '0')}E${String(ep.number).padStart(2, '0')}`
}

function handlePlayFromStart() {
  const ep = firstEpisode()
  if (!ep) return
  emit('play', ep.path, null)
  emit('toggle-menu', menuKey.value)
}

function handleResume() {
  const ep = firstUnwatchedEpisode()
  if (!ep) return
  const startPosition = ep.position && ep.position > 0 ? ep.position : null
  emit('play', ep.path, startPosition)
  emit('toggle-menu', menuKey.value)
}

function handleMarkWatched() {
  emit('mark-watched', props.show, !allWatched())
  emit('toggle-menu', menuKey.value)
}

function handleMatch() {
  emit('match', props.show)
  emit('toggle-menu', menuKey.value)
}
</script>

<template>
  <article class="card" @click="emit('open', show)">
    <MediaPoster
      :poster-url="show.tmdb?.poster_local || null"
      :alt="show.title"
      :rating="show.tmdb?.rating ?? null"
      :watched="allWatched()"
      :progress="showProgress()"
      placeholder="Сериал"
    >
      <template #overlay>
        <button class="menu-btn" @click.stop="emit('toggle-menu', menuKey)">⋮</button>
        <div v-if="menuOpen" class="context-menu" @click.stop>
          <button @click="handlePlayFromStart">Смотреть с начала</button>
          <button v-if="firstUnwatchedEpisode()" @click="handleResume">
            Продолжить с {{ episodeLabel(firstUnwatchedEpisode()!) }}
          </button>
          <button v-if="!allWatched()" @click="handleMarkWatched">Пометить просмотренным</button>
          <button v-else @click="handleMarkWatched">Непросмотренно</button>
          <button @click="handleMatch">Сопоставить</button>
        </div>
      </template>
    </MediaPoster>

    <div class="card-title">{{ show.title }}</div>
    <div class="card-year">
      <template v-if="show.year">{{ show.year }} · </template>
      <template v-if="allWatched()">
        <span class="all-watched">✓ Все просмотрено</span>
      </template>
      <template v-else>
        <span v-if="episodesWatched() === 0">{{ episodesTotal() }} сер.</span>
        <span v-else>
          Осталось {{ episodesTotal() - episodesWatched() }} из {{ episodesTotal() }}
        </span>
      </template>
    </div>
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
