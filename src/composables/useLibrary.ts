import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Library } from '@/types'

export function useLibrary() {
  const library = ref<Library>({ movies: [], tv_shows: [] })
  const loading = ref(true)
  const error = ref<string | null>(null)
  const scanProgress = ref<{ current: number; total: number; folder: string } | null>(null)

  async function refresh() {
    loading.value = true
    error.value = null
    scanProgress.value = null
    try {
      library.value = await invoke<Library>('scan_all')
    } catch (e) {
      error.value = String(e)
      library.value = { movies: [], tv_shows: [] }
    } finally {
      loading.value = false
      scanProgress.value = null
    }
  }

  return {
    library,
    loading,
    error,
    scanProgress,
    refresh,
  }
}