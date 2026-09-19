<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

type MediaType = 'movie' | 'tv_shows'

interface FolderConfig {
  path: string
  type: MediaType
}

const folders = ref<FolderConfig[]>([])
const newType = ref<MediaType>('movie')
const error = ref<string | null>(null)
const emit = defineEmits<{ (e: 'back'): void }>()

async function loadFolders() {
  folders.value = await invoke<FolderConfig[]>('get_folders')
}

async function addFolder() {
  const path = await open({
    directory: true,
    multiple: false,
    title: 'Выберите папку'
  })
  if (!path) return

  try {
    folders.value = await invoke<FolderConfig[]>('add_folder', {
      path,
      mediaType: newType.value
    })
    error.value = null
  } catch (e) {
    error.value = String(e)
  }
}

async function removeFolder(path: string) {
  folders.value = await invoke<FolderConfig[]>('remove_folder', { path })
}

onMounted(loadFolders)
</script>

<template>
  <div class="settings">
    <h2>Настройки</h2>

    <section class="add-section">
      <h3>Добавить папку</h3>
      <div class="add-row">
        <select v-model="newType">
          <option value="movie">Фильмы</option>
          <option value="tv_shows">Сериалы</option>
        </select>
        <button @click="addFolder">Выбрать папку</button>
      </div>
    </section>

    <p v-if="error" class="error">{{ error }}</p>

    <section class="list-section">
      <h3>Папки в библиотеке</h3>
      <p v-if="!folders.length" class="empty">Пока нет добавленных папок</p>
      <ul v-else class="folders">
        <li v-for="folder in folders" :key="folder.path">
          <div class="folder-info">
            <span class="badge" :class="folder.type">
              {{ folder.type === 'movie' ? 'Фильмы' : 'Сериалы' }}
            </span>
            <span class="path">{{ folder.path }}</span>
          </div>
          <button class="remove" @click="removeFolder(folder.path)">Удалить</button>
        </li>
      </ul>
    </section>
  </div>
</template>

<style scoped>
.settings {
  max-width: 800px;
}
h2 {
  margin-top: 0;
}
h3 {
  margin: 1.5rem 0 0.75rem;
  font-size: 1rem;
  color: #aaa;
  font-weight: 500;
}
.add-row {
  display: flex;
  gap: 0.75rem;
  align-items: center;
}
select {
  background: #2a2e35;
  color: #e6e6e6;
  border: 1px solid #3a3f47;
  padding: 0.5rem 0.75rem;
  border-radius: 6px;
  font-size: 0.9rem;
}
button {
  background: #2a2e35;
  color: #e6e6e6;
  border: 1px solid #3a3f47;
  padding: 0.5rem 1rem;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.9rem;
}
button:hover {
  background: #353a42;
}
.error {
  color: #ff6b6b;
}
.empty {
  color: #666;
}
.folders {
  list-style: none;
  padding: 0;
  margin: 0;
}
.folders li {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem 0;
  border-bottom: 1px solid #2a2e35;
}
.folder-info {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  min-width: 0;
}
.badge {
  padding: 0.2rem 0.6rem;
  border-radius: 4px;
  font-size: 0.75rem;
  flex-shrink: 0;
}
.badge.movie {
  background: #2d4a6b;
  color: #9dc7f0;
}
.badge.tv_shows {
  background: #4a2d6b;
  color: #c79df0;
}
.path {
  font-family: monospace;
  font-size: 0.85rem;
  color: #aaa;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.remove {
  background: transparent;
  border: 1px solid #4a2d2d;
  color: #ff8888;
  padding: 0.3rem 0.75rem;
  font-size: 0.8rem;
}
.remove:hover {
  background: #4a2d2d;
}
</style>
