<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

interface VideoFile {
  path: string;
  name: string;
  extension: string;
}

const videos = ref<VideoFile[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

async function selectFolder() {
  const folder = await open({
    directory: true,
    multiple: false,
    title: "Выберите папку с видео",
  });

  if (!folder) return;

  loading.value = true;
  error.value = null;

  try {
    videos.value = await invoke<VideoFile[]>("scan_videos", {
      folderPath: folder,
    });
  } catch (e) {
    error.value = String(e);
    videos.value = [];
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <main class="container">
    <h1>Lumi</h1>
    <button @click="selectFolder" :disabled="loading">
      {{ loading ? "Сканирование..." : "Выбрать папку" }}
    </button>

    <p v-if="error" class="error">{{ error }}</p>
    <p v-else-if="videos.length">Найдено файлов: {{ videos.length }}</p>

    <ul v-if="videos.length">
      <li v-for="video in videos" :key="video.path">
        {{ video.name }}
      </li>
    </ul>
  </main>
</template>

<style scoped>
.container {
  padding: 2rem;
  font-family: sans-serif;
}
.error {
  color: red;
}
</style>