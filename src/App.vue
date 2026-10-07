<script setup lang="ts">
import { ref } from 'vue'
import Settings from './views/Settings.vue'
import LibraryView from './views/LibraryView.vue'
import DetailView from './views/DetailView.vue'
import { useKeyboard } from '@/composables/useKeyboard'
import '@/styles/modal.css'
import type { VideoFile, TvShow } from '@/types'
import { usePlayer } from '@/composables/usePlayer'

const currentView = ref<'library' | 'settings' | 'detail'>('library')
const detailItem = ref<VideoFile | TvShow | null>(null)
const { play } = usePlayer()

function openDetail(item: VideoFile | TvShow) {
  detailItem.value = item
  currentView.value = 'detail'
}

function closeDetail() {
  detailItem.value = null
  currentView.value = 'library'
}

useKeyboard({ currentView, withFullscreen: true })
</script>

<template>
  <!-- Detail view: полный экран, без padding и toolbar -->
  <DetailView v-if="currentView === 'detail' && detailItem" :item="detailItem" @close="closeDetail" />

  <!-- Library / Settings: обычный layout с toolbar -->
  <main v-else class="app">
    <header class="toolbar">
      <div class="header-left">
        <button v-if="currentView === 'settings'" class="back-arrow" @click="currentView = 'library'"
          title="Назад (Backspace)">
          ←
        </button>
        <h1>Lumi</h1>
      </div>
      <nav class="tabs">
        <button :class="{ active: currentView === 'library' }" @click="currentView = 'library'">
          Библиотека
        </button>
        <button :class="{ active: currentView === 'settings' }" @click="currentView = 'settings'">
          Настройки
        </button>
      </nav>
    </header>

    <Settings v-if="currentView === 'settings'" />
    <LibraryView v-else @open-movie="openDetail" @open-show="openDetail" />
  </main>
</template>

<style>
* {
  box-sizing: border-box;
}

body {
  margin: 0;
  background: #14161a;
  color: #e6e6e6;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}

.app {
  min-height: 100vh;
  padding: 1.5rem 2rem;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 1.5rem;
}

.toolbar h1 {
  margin: 0;
  font-size: 1.5rem;
  letter-spacing: 0.05em;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.back-arrow {
  background: transparent;
  border: none;
  color: #aaa;
  font-size: 1.5rem;
  cursor: pointer;
  padding: 0 0.5rem;
  line-height: 1;
  transition: color 0.15s ease;
}

.back-arrow:hover {
  color: #fff;
}

.tabs {
  display: flex;
  gap: 0.5rem;
}

.tabs button {
  background: transparent;
  border: none;
  color: #888;
  padding: 0.5rem 1rem;
  cursor: pointer;
  border-radius: 6px;
  font-size: 0.9rem;
}

.tabs button:hover {
  color: #e6e6e6;
}

.tabs button.active {
  background: #2a2e35;
  color: #e6e6e6;
}
</style>