import { invoke } from '@tauri-apps/api/core'
import type { VideoFile, TvShow } from '@/types'

export function useWatched(onAfterChange: () => Promise<void>) {
  async function markMovie(movie: VideoFile, watched: boolean) {
    await invoke('set_watched_bulk', { paths: [movie.path], watched })
    movie.watched = watched
    movie.position = 0
    movie.duration = 0
    await onAfterChange()
  }

  async function markShow(show: TvShow, watched: boolean) {
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
    await onAfterChange()
  }

  return { markMovie, markShow }
}