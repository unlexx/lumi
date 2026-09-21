import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import type { Ref } from 'vue'
import type { Library } from '@/types'

export function usePosters(library: Ref<Library>) {
  async function load() {
    for (const movie of library.value.movies) {
      if (!movie.tmdb?.poster_url) continue
      try {
        const localPath = await invoke<string>('get_poster', {
          tmdbId: movie.tmdb.id,
          posterPath: extractPosterPath(movie.tmdb.poster_url),
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
          posterPath: extractPosterPath(show.tmdb.poster_url),
        })
        show.tmdb.poster_local = convertFileSrc(localPath)
      } catch (e) {
        console.error('Poster error for', show.title, e)
      }
    }
  }

  return { load }
}

function extractPosterPath(url: string): string {
  const marker = '/t/p/w500'
  const idx = url.indexOf(marker)
  return idx >= 0 ? url.slice(idx + marker.length) : url
}