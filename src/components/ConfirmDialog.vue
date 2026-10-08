<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'

interface Props {
    title?: string
    message?: string
    confirmLabel?: string
    cancelLabel?: string
    danger?: boolean
}

const props = withDefaults(defineProps<Props>(), {
    title: 'Подтвердите действие',
    message: '',
    confirmLabel: 'OK',
    cancelLabel: 'Отмена',
    danger: false,
})

const emit = defineEmits<{
    (e: 'confirm'): void
    (e: 'cancel'): void
}>()

const confirmBtn = ref<HTMLButtonElement | null>(null)

function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
        e.preventDefault()
        e.stopPropagation()
        emit('cancel')
    } else if (e.key === 'Enter') {
        e.preventDefault()
        emit('confirm')
    }
}

onMounted(() => {
    confirmBtn.value?.focus()
    window.addEventListener('keydown', onKeydown, { capture: true })
})

onUnmounted(() => {
    window.removeEventListener('keydown', onKeydown, { capture: true })
})
</script>

<template>
    <div class="confirm-overlay" @click.self="emit('cancel')">
        <div class="confirm-dialog" role="dialog" aria-modal="true">
            <h3 class="confirm-title">{{ props.title }}</h3>
            <p v-if="props.message" class="confirm-message">{{ props.message }}</p>
            <div class="confirm-actions">
                <button class="btn-cancel" @click="emit('cancel')">
                    {{ props.cancelLabel }}
                </button>
                <button ref="confirmBtn" class="btn-confirm" :class="{ danger: props.danger }" @click="emit('confirm')">
                    {{ props.confirmLabel }}
                </button>
            </div>
        </div>
    </div>
</template>

<style scoped>
.confirm-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
}

.confirm-dialog {
    background: #1e2228;
    border: 1px solid #2f343c;
    border-radius: 10px;
    padding: 1.5rem;
    min-width: 360px;
    max-width: 480px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
}

.confirm-title {
    margin: 0 0 0.75rem;
    font-size: 1.1rem;
    color: #e6e6e6;
}

.confirm-message {
    margin: 0 0 1.25rem;
    color: #aaa;
    font-size: 0.9rem;
    line-height: 1.5;
}

.confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
}

.confirm-actions button {
    background: #2a2e35;
    color: #e6e6e6;
    border: 1px solid #3a3f47;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
}

.confirm-actions button:hover {
    background: #353a42;
}

.btn-confirm.danger {
    background: #6b2d2d;
    border-color: #7b3d3d;
}

.btn-confirm.danger:hover {
    background: #7b3d3d;
}

.btn-confirm:focus {
    outline: 2px solid #5a8bd8;
    outline-offset: 1px;
}
</style>