<script setup lang="ts">
import { computed } from 'vue';
import { useDragDrop } from '../composables/useDragDrop';
import { open } from '@tauri-apps/plugin-dialog';

const props = defineProps<{
  label: string;
  icon: string;
  modelValue: string;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

const { isDragging, onDragOver, onDragLeave, onDrop } = useDragDrop();

const displayPath = computed(() => props.modelValue || '拖拽文件夹到此处');

function handleDragOver(e: DragEvent) {
  if (props.disabled) return;
  onDragOver(e);
}

function handleDragLeave(e: DragEvent) {
  onDragLeave(e);
}

function handleDrop(e: DragEvent) {
  if (props.disabled) return;
  const path = onDrop(e);
  if (path) {
    emit('update:modelValue', path);
  }
}

async function browse() {
  if (props.disabled) return;
  const selected = await open({ directory: true, multiple: false });
  if (selected) {
    emit('update:modelValue', selected);
  }
}
</script>

<template>
  <div
    class="drop-zone"
    :class="{ active: isDragging && !disabled, 'has-path': modelValue, disabled }"
    @dragover="handleDragOver"
    @dragleave="handleDragLeave"
    @drop="handleDrop"
    @click="browse"
  >
    <div class="drop-icon">{{ icon }}</div>
    <div class="drop-label">{{ label }}</div>
    <div class="drop-hint">{{ displayPath }}</div>
  </div>
</template>

<style scoped>
.drop-zone {
  border: 2px dashed var(--border-color);
  border-radius: 10px;
  padding: 24px 16px;
  text-align: center;
  background: var(--surface-color);
  cursor: pointer;
  transition: all 0.2s ease;
}

.drop-zone:hover:not(.disabled) {
  border-color: var(--primary-color);
  background: var(--surface-hover);
}

.drop-zone.active {
  border-color: var(--primary-color);
  background: var(--primary-dim);
  transform: scale(1.02);
}

.drop-zone.has-path {
  border-color: var(--success-color);
  border-style: solid;
}

.drop-zone.disabled {
  opacity: 0.5;
  cursor: not-allowed;
  pointer-events: auto;
}

.drop-icon {
  font-size: 32px;
  margin-bottom: 8px;
}

.drop-label {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 4px;
  color: var(--text-primary);
}

.drop-hint {
  font-size: 11px;
  color: var(--text-secondary);
  word-break: break-all;
  max-height: 32px;
  overflow: hidden;
}
</style>
