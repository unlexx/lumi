import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Ref } from 'vue'
import type { Library, ContinueItem } from '@/types'

export function useContinueWatching(library: Ref<Library>) {
  const continueWatching = ref<ContinueItem[]>([])

  async function load() {
    const allPaths: string[] = [
      ...library.value.movies.map((m) => m.path),
      ...library.value.tv_shows.flatMap((s) =>
        s.seasons.flatMap((se) => se.episodes.map((e) => e.path))
      ),
    ]

    const raw = await invoke<ContinueItem[]>('get_continue_watching', {
      paths: allPaths,
    })

    continueWatching.value = raw
      .map((item) => {
        const movie = library.value.movies.find((m) => m.path === item.path)
        if (movie) {
          return {
            ...item,
            title: movie.tmdb?.title || movie.parsed.title,
            poster_url: movie.tmdb?.poster_local || null,
            media_type: 'movie' as const,
          }
        }

        for (const show of library.value.tv_shows) {
          for (const season of show.seasons) {
            const ep = season.episodes.find((e) => e.path === item.path)
            if (ep) {
              return {
                ...item,
                title: `${show.title} — S${String(season.number).padStart(2, '0')}E${String(ep.number).padStart(2, '0')}`,
                poster_url: show.tmdb?.poster_local || null,
                media_type: 'tv_shows' as const,
              }
            }
          }
        }

        return item
      })
      .filter((item) => item.poster_url || item.title)
  }

  return { continueWatching, load }
}