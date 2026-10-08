import { onKeyStroke } from '@vueuse/core'
import { invoke } from '@tauri-apps/api/core'
import type { Ref } from 'vue'

interface KeyboardOptions {
  currentView?: Ref<'library' | 'settings' | 'detail'>
  onEscape?: () => void
  withFullscreen?: boolean
}

function isEditableTarget(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null
  if (!el) return false
  const tag = el.tagName
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || el.isContentEditable
}

export function useKeyboard(options: KeyboardOptions = {}) {
  if (options.currentView) {
    onKeyStroke('Backspace', (e) => {
      if (isEditableTarget(e.target)) return
      if (options.currentView!.value === 'settings') {
        e.preventDefault()
        options.currentView!.value = 'library'
      }
    })
  }

  if (options.withFullscreen) {
    onKeyStroke('F11', (e) => {
      e.preventDefault()
      invoke('toggle_fullscreen').catch(console.error)
    })
  }

  if (options.onEscape) {
    onKeyStroke('Escape', (e) => {
      // Не мешаем input'ам обрабатывать Escape самим
      if (isEditableTarget(e.target)) return
      options.onEscape!()
    })
  }
}
