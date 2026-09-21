import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

export function usePlayer() {
  const error = ref<string | null>(null)

  async function play(path: string, startPosition: number | null = null) {
    try {
      await invoke('play_video', { path, startPosition })
    } catch (e) {
      console.error('Play error:', e)
      error.value = String(e)
    }
  }

  return { play, error }
}