<script setup lang="ts">
import type { ConvertDoneEvent, FileEntry, OutputSettings } from '../types';
import { SAMPLE_RATE_OPTIONS, BIT_RATE_OPTIONS } from '../types';

defineProps<{
  files: FileEntry[];
  isConverting: boolean;
  outputDir: string;
  doneSummary: ConvertDoneEvent | null;
  settings: OutputSettings;
}>();

const emit = defineEmits<{
  start: [];
  openFolder: [];
  reset: [];
}>();
</script>

<template>
  <div class="action-bar">
    <div class="action-summary">
      <template v-if="doneSummary">
        进度：{{ doneSummary.success }}/{{ doneSummary.total }} 完成
        <span v-if="doneSummary.failed > 0" class="failed-count">，{{ doneSummary.failed }} 失败</span>
      </template>
      <template v-else-if="isConverting">
        <span class="converting-spinner"></span>
        正在转换中...
      </template>
      <template v-else-if="files.length > 0">
        已就绪，{{ files.length }} 个文件等待转换
      </template>
    </div>
    <div class="action-controls">
      <div class="settings-group">
        <div class="setting-item">
          <label class="setting-label">采样率</label>
          <select
            :value="settings.sampleRate"
            class="setting-select"
            :disabled="isConverting"
            @change="settings.sampleRate = Number(($event.target as HTMLSelectElement).value)"
          >
            <option v-for="opt in SAMPLE_RATE_OPTIONS" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
          </select>
        </div>
        <div class="setting-item">
          <label class="setting-label">比特率</label>
          <select
            :value="settings.bitRate"
            class="setting-select"
            :disabled="isConverting"
            @change="settings.bitRate = Number(($event.target as HTMLSelectElement).value)"
          >
            <option v-for="opt in BIT_RATE_OPTIONS" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
          </select>
        </div>
      </div>
      <div class="action-buttons">
        <button
          v-if="outputDir && doneSummary"
          class="btn btn-secondary"
          @click="emit('openFolder')"
        >
          打开输出文件夹
        </button>
        <button
          v-if="doneSummary"
          class="btn btn-primary"
          @click="emit('reset')"
        >
          重新开始
        </button>
        <button
          v-if="files.length > 0 && !isConverting && !doneSummary"
          class="btn btn-primary"
          @click="emit('start')"
        >
          开始转换
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.action-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 12px;
  flex-wrap: wrap;
  gap: 8px;
}

.action-summary {
  font-size: 12px;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  gap: 6px;
}

.converting-spinner {
  display: inline-block;
  width: 12px;
  height: 12px;
  border: 2px solid var(--primary-dim);
  border-top-color: var(--primary-color);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  flex-shrink: 0;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.failed-count {
  color: var(--error-color);
}

.action-controls {
  display: flex;
  align-items: center;
  gap: 12px;
}

.settings-group {
  display: flex;
  gap: 10px;
}

.setting-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.setting-label {
  font-size: 11px;
  color: var(--text-secondary);
  white-space: nowrap;
}

.setting-select {
  background: var(--surface-color);
  color: var(--text-primary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 3px 6px;
  font-size: 11px;
  cursor: pointer;
  outline: none;
  transition: border-color 0.2s;
}

.setting-select:hover:not(:disabled) {
  border-color: var(--text-secondary);
}

.setting-select:focus {
  border-color: var(--primary-color);
}

.setting-select:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.action-buttons {
  display: flex;
  gap: 8px;
}

.btn {
  padding: 6px 14px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  border: none;
  transition: all 0.2s;
}

.btn-primary {
  background: var(--success-color);
  color: #000;
}

.btn-primary:hover {
  filter: brightness(1.1);
}

.btn-secondary {
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border-color);
}

.btn-secondary:hover {
  border-color: var(--text-secondary);
}
</style>
