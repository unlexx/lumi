<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

interface ParsedVideo {
  title: string;
  year: number | null;
  resolution: string | null;
  source: string | null;
  codec: string | null;
}

interface VideoFile {
  path: string;
  name: string;
  extension: string;
  parsed: ParsedVideo;
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

    <ul v-if="videos.length" class="video-list">
      <li v-for="video in videos" :key="video.path">
        <div class="title">
          {{ video.parsed.title }}
          <span v-if="video.parsed.year" class="year">({{ video.parsed.year }})</span>
        </div>
        <div class="meta">
          <span v-if="video.parsed.resolution">{{ video.parsed.resolution }}</span>
          <span v-if="video.parsed.source">{{ video.parsed.source }}</span>
          <span v-if="video.parsed.codec">{{ video.parsed.codec }}</span>
        </div>
        <div class="raw">{{ video.name }}</div>
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
.video-list {
  list-style: none;
  padding: 0;
}
.video-list li {
  padding: 1rem 0;
  border-bottom: 1px solid #eee;
}
.title {
  font-size: 1.1rem;
  font-weight: 600;
}
.year {
  color: #888;
  font-weight: 400;
}
.meta {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.25rem;
  color: #666;
  font-size: 0.85rem;
}
.raw {
  margin-top: 0.25rem;
  color: #aaa;
  font-size: 0.75rem;
  font-family: monospace;
}
</style>