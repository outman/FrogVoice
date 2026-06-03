<script setup lang="ts">
import type { ConvertDoneEvent, FileEntry } from '../types';

defineProps<{
  files: FileEntry[];
  isConverting: boolean;
  outputDir: string;
  doneSummary: ConvertDoneEvent | null;
}>();

const emit = defineEmits<{
  start: [];
  openFolder: [];
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
        正在转换中...
      </template>
      <template v-else-if="files.length > 0">
        已就绪，{{ files.length }} 个文件等待转换
      </template>
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
        v-if="files.length > 0 && !isConverting && !doneSummary"
        class="btn btn-primary"
        @click="emit('start')"
      >
        开始转换
      </button>
    </div>
  </div>
</template>

<style scoped>
.action-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 12px;
}

.action-summary {
  font-size: 12px;
  color: var(--text-secondary);
}

.failed-count {
  color: var(--error-color);
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
