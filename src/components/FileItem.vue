<script setup lang="ts">
import type { FileEntry } from '../types';
import { formatDuration, formatFileSize, getStatusText, getStatusClass } from '../types';

defineProps<{ entry: FileEntry }>();
</script>

<template>
  <div class="file-item" :class="getStatusClass(entry.status)">
    <div class="file-info">
      <div class="file-name">🎵 {{ entry.file.name }}</div>
      <div class="file-meta">
        {{ formatDuration(entry.file.duration_secs) }} · {{ formatFileSize(entry.file.size) }}
      </div>
    </div>
    <div class="file-format">192kbps → MP3</div>
    <div class="file-status">
      <template v-if="entry.status.type === 'Converting'">
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: (entry.status.progress * 100) + '%' }"></div>
        </div>
        <span class="status-text converting">{{ getStatusText(entry.status) }}</span>
      </template>
      <span v-else class="status-text" :class="getStatusClass(entry.status)">
        {{ getStatusText(entry.status) }}
      </span>
    </div>
    <div class="file-output">
      <template v-if="entry.status.type === 'Completed'">
        {{ formatFileSize(entry.status.output_size) }}
      </template>
      <template v-else>—</template>
    </div>
  </div>
</template>

<style scoped>
.file-item {
  display: grid;
  grid-template-columns: 1fr 100px 120px 70px;
  align-items: center;
  padding: 10px 16px;
  border-bottom: 1px solid var(--border-color);
  border-left: 3px solid var(--border-color);
  transition: background 0.2s;
}

.file-item:last-child {
  border-bottom: none;
}

.file-item.status-pending {
  border-left-color: var(--text-secondary);
}
.file-item.status-converting {
  border-left-color: var(--primary-color);
  background: var(--primary-dim);
}
.file-item.status-completed {
  border-left-color: var(--success-color);
  background: var(--success-dim);
}
.file-item.status-failed {
  border-left-color: var(--error-color);
  background: var(--error-dim);
}

.file-info {
  min-width: 0;
}

.file-name {
  font-size: 13px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.file-meta {
  font-size: 11px;
  color: var(--text-secondary);
  margin-top: 2px;
}

.file-format {
  font-size: 11px;
  color: var(--text-secondary);
}

.file-status {
  font-size: 11px;
}

.file-output {
  font-size: 11px;
  color: var(--text-secondary);
  text-align: right;
}

.progress-bar {
  background: var(--border-color);
  border-radius: 3px;
  height: 4px;
  width: 80px;
  margin-bottom: 4px;
}

.progress-fill {
  background: var(--primary-color);
  height: 100%;
  border-radius: 3px;
  transition: width 0.2s;
}

.status-text.status-completed {
  color: var(--success-color);
}

.status-text.status-failed {
  color: var(--error-color);
}

.status-text.converting {
  color: var(--primary-color);
}
</style>
