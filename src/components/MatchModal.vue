<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { TmdbSearchResult } from '@/types'

interface MatchTarget {
  media_type: 'movie' | 'tv_shows'
  title: string
  uids: string[]
}

const props = defineProps<{
  target: MatchTarget
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'apply', result: TmdbSearchResult): void
}>()

const query = ref(props.target.title)
const results = ref<TmdbSearchResult[]>([])
const loading = ref(false)

async function search() {
  loading.value = true
  try {
    const found = await invoke<TmdbSearchResult[]>('search_tmdb_manual', {
      query: query.value,
      mediaType: props.target.media_type
    })
    results.value = found

    // Постеры в фоне
    for (const result of results.value) {
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
    loading.value = false
  }
}

// Если target меняется (например, другая карточка), перезапускаем поиск
watch(
  () => props.target,
  (next) => {
    query.value = next.title
    results.value = []
    search()
  }
)

onMounted(() => {
  search()
})
</script>

<template>
  <div class="modal-backdrop" @mousedown.self="emit('close')">
    <div class="modal match-modal">
      <button class="close" @click="emit('close')">×</button>
      <h2>Сопоставить: {{ target.title }}</h2>

      <div class="search-row">
        <input
          v-model="query"
          type="text"
          placeholder="Название фильма или сериала"
          @keydown.enter="search"
        />
        <button :disabled="loading" @click="search">
          {{ loading ? 'Поиск...' : 'Искать' }}
        </button>
      </div>

      <div v-if="results.length" class="results">
        <article
          v-for="result in results"
          :key="result.id"
          class="result-item"
          @click="emit('apply', result)"
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
      <p v-else-if="!loading && query" class="no-results">
        Ничего не найдено. Попробуйте другое название.
      </p>
    </div>
  </div>
</template>

<style scoped>
/* Стили, специфичные для MatchModal.
   .modal-backdrop / .modal / .close — пока остаются глобальными в App.vue,
   чтобы не трогать MovieModal/ShowModal до их выноса. */
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
</style>