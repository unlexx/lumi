import { onKeyStroke } from '@vueuse/core'
import { invoke } from '@tauri-apps/api/core'
import type { Ref } from 'vue'

export function useKeyboard(options: {
  currentView: Ref<'library' | 'settings'>
  selectedVideo: Ref<any>
  selectedShow: Ref<any>
}) {
  onKeyStroke('Backspace', (e) => {
    if (options.currentView.value === 'settings') {
      e.preventDefault()
      options.currentView.value = 'library'
    }
  })

  onKeyStroke('F11', (e) => {
    e.preventDefault()
    invoke('toggle_fullscreen').catch(console.error)
  })

  onKeyStroke('Escape', () => {
    if (options.selectedVideo.value) {
      options.selectedVideo.value = null
    } else if (options.selectedShow.value) {
      options.selectedShow.value = null
    } else {
      invoke('toggle_fullscreen').catch(console.error)
    }
  })
}
