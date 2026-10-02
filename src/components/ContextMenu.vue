<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'

const emit = defineEmits<{
  (e: 'open'): void
  (e: 'close'): void
}>()

const isOpen = ref(false)
const rootEl = ref<HTMLElement | null>(null)

function open() {
  if (isOpen.value) return
  isOpen.value = true
  emit('open')
}

function close() {
  if (!isOpen.value) return
  isOpen.value = false
  emit('close')
}

function toggle() {
  isOpen.value ? close() : open()
}

function onDocumentClick(event: MouseEvent) {
  if (!isOpen.value) return
  const target = event.target as Node | null
  if (rootEl.value && target && !rootEl.value.contains(target)) {
    close()
  }
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') close()
}

onMounted(() => {
  document.addEventListener('mousedown', onDocumentClick)
  document.addEventListener('keydown', onKeydown)
})

onBeforeUnmount(() => {
  document.removeEventListener('mousedown', onDocumentClick)
  document.removeEventListener('keydown', onKeydown)
})

defineExpose({ open, close, toggle })
</script>

<template>
  <div ref="rootEl" class="context-menu-root">
    <slot name="trigger" :open="open" :close="close" :toggle="toggle" :is-open="isOpen" />
    <div v-if="isOpen" class="context-menu" @click.stop>
      <slot name="menu" :close="close" />
    </div>
  </div>
</template>

<style scoped>
.context-menu-root {
  position: relative;
}

.context-menu {
  position: absolute;
  bottom: 38px;
  right: 6px;
  background: #1e2127;
  border: 1px solid #3a3f47;
  border-radius: 6px;
  padding: 0.25rem 0;
  min-width: 120px;
  max-width: 240px;
  z-index: 10;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
}

.context-menu :deep(button) {
  display: block;
  width: 100%;
  background: transparent;
  border: none;
  color: #e6e6e6;
  text-align: left;
  padding: 0.5rem 1rem;
  cursor: pointer;
  font-size: 0.85rem;
  white-space: normal;
  word-break: break-word;
}

.context-menu :deep(button:hover) {
  background: #2a2e35;
}
</style>