import { onKeyStroke } from '@vueuse/core'
import { invoke } from '@tauri-apps/api/core'
import type { Ref } from 'vue'

interface KeyboardOptions {
  currentView?: Ref<'library' | 'settings'>
  onEscape?: () => void
  withFullscreen?: boolean
}

export function useKeyboard(options: KeyboardOptions = {}) {
  if (options.currentView) {
    onKeyStroke('Backspace', (e) => {
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
    onKeyStroke('Escape', () => {
      options.onEscape!()
    })
  }
}
