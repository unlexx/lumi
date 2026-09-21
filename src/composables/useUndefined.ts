import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { UndefinedItem } from '@/types'

export function useUndefined() {
  const items = ref<UndefinedItem[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function load() {
    loading.value = true
    error.value = null
    try {
      items.value = await invoke<UndefinedItem[]>('get_undefined_items')
    } catch (e) {
      error.value = String(e)
      items.value = []
    } finally {
      loading.value = false
    }
  }

  function remove(uid: string) {
    items.value = items.value.filter((i) => i.uid !== uid)
  }

  return {
    items,
    loading,
    error,
    load,
    remove
  }
}
