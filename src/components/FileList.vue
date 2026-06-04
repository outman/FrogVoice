<script setup lang="ts">
import type { FileEntry } from '../types';
import FileItem from './FileItem.vue';

defineProps<{ files: FileEntry[]; bitRate: number }>();
</script>

<template>
  <div class="file-list" v-if="files.length > 0">
    <div class="file-list-header">
      <span class="header-title">待转换文件 <span class="header-count">({{ files.length }} 个文件)</span></span>
    </div>
    <div class="file-list-body">
      <FileItem v-for="entry in files" :key="entry.file.path" :entry="entry" :bit-rate="bitRate" />
    </div>
  </div>
  <div v-else class="file-list-empty">
    <p>请选择输入文件夹以扫描音频文件</p>
  </div>
</template>

<style scoped>
.file-list {
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.file-list-header {
  padding: 10px 16px;
  background: var(--surface-color);
  border-bottom: 1px solid var(--border-color);
}

.header-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.header-count {
  font-weight: 400;
  color: var(--text-secondary);
}

.file-list-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.file-list-empty {
  text-align: center;
  padding: 32px;
  color: var(--text-secondary);
  border: 1px dashed var(--border-color);
  border-radius: 8px;
  font-size: 14px;
}
</style>
