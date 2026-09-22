<script setup lang="ts">
import type { TvShow, Episode } from '@/types'
import MediaPoster from './MediaPoster.vue'
import ContextMenu from './ContextMenu.vue'

const props = defineProps<{
  show: TvShow
}>()

const emit = defineEmits<{
  (e: 'open', show: TvShow): void
  (e: 'play', path: string, startPosition: number | null): void
  (e: 'match', show: TvShow): void
  (e: 'mark-watched', show: TvShow, watched: boolean): void
}>()

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
        <ContextMenu>
          <template #trigger="{ toggle }">
            <button class="menu-btn" @click.stop="toggle">⋮</button>
          </template>
          <template #menu="{ close }">
            <button
              @click="
                () => {
                  const ep = firstEpisode()
                  if (ep) emit('play', ep.path, null)
                  close()
                }
              "
            >
              Смотреть с начала
            </button>
            <button
              v-if="firstUnwatchedEpisode()"
              @click="
                () => {
                  const ep = firstUnwatchedEpisode()!
                  emit('play', ep.path, ep.position && ep.position > 0 ? ep.position : null)
                  close()
                }
              "
            >
              Продолжить с {{ episodeLabel(firstUnwatchedEpisode()!) }}
            </button>
            <button
              v-if="!allWatched()"
              @click="
                () => {
                  emit('mark-watched', show, true)
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
                  emit('mark-watched', show, false)
                  close()
                }
              "
            >
              Непросмотренно
            </button>
            <button
              @click="
                () => {
                  emit('match', show)
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
</style>