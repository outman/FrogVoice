<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import DropZone from './components/DropZone.vue';
import FileList from './components/FileList.vue';
import ActionBar from './components/ActionBar.vue';
import { useConverter } from './composables/useConverter';

const inputDir = ref('');
const outputDir = ref('');

const {
  files,
  isConverting,
  isScanning,
  doneSummary,
  error,
  scanFiles,
  startConversion,
  openFolder,
  reset,
  resetConversion,
} = useConverter();

const isLoading = computed(() => isScanning.value || isConverting.value);

// When inputDir changes, scan for files
watch(inputDir, async (newDir) => {
  if (newDir) {
    await scanFiles(newDir);
  } else {
    reset();
  }
});

async function handleStart() {
  if (!outputDir.value) return;
  await startConversion(outputDir.value);
}

function handleOpenFolder() {
  if (outputDir.value) {
    openFolder(outputDir.value);
  }
}
</script>

<template>
  <div class="app" @dragover.prevent @drop.prevent>
    <header class="app-header">
      <h1 class="app-title">🐸 FrogVoice</h1>
      <p class="app-subtitle">音频批量转 MP3 工具</p>
    </header>

    <div class="drop-zones">
      <DropZone
        v-model="inputDir"
        label="输入文件夹"
        icon="📁"
        :disabled="isLoading"
      />
      <DropZone
        v-model="outputDir"
        label="输出文件夹"
        icon="📂"
        :disabled="isLoading"
      />
    </div>

    <!-- Scanning overlay -->
    <div v-if="isScanning" class="loading-banner">
      <span class="loading-spinner"></span>
      正在扫描音频文件，请稍候...
    </div>

    <div v-if="error" class="error-banner">{{ error }}</div>

    <div class="content">
      <FileList :files="files" class="file-list-wrapper" />
      <ActionBar
        :files="files"
        :is-converting="isConverting"
        :output-dir="outputDir"
        :done-summary="doneSummary"
        @start="handleStart"
        @open-folder="handleOpenFolder"
        @reset="resetConversion"
      />
    </div>
  </div>
</template>

<style>
/* Global styles — dark theme */
:root {
  --bg-color: #1a1a2e;
  --surface-color: #16213e;
  --surface-hover: #1c2a4a;
  --border-color: #2a2a4a;
  --text-primary: #e0e0e0;
  --text-secondary: #888;
  --primary-color: #3b82f6;
  --primary-dim: rgba(59, 130, 246, 0.08);
  --success-color: #28c840;
  --success-dim: rgba(40, 200, 64, 0.08);
  --error-color: #ef4444;
  --error-dim: rgba(239, 68, 68, 0.08);
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC',
    'Hiragino Sans GB', 'Microsoft YaHei', sans-serif;
  background: var(--bg-color);
  color: var(--text-primary);
  overflow: hidden;
  user-select: none;
}

/* Scrollbar */
::-webkit-scrollbar {
  width: 6px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 3px;
}
</style>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  padding: 20px;
  gap: 16px;
}

.app-header {
  text-align: center;
}

.app-title {
  font-size: 20px;
  font-weight: 700;
}

.app-subtitle {
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 2px;
}

.drop-zones {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

.loading-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  background: var(--primary-dim);
  border: 1px solid var(--primary-color);
  border-radius: 6px;
  padding: 10px 14px;
  font-size: 13px;
  color: var(--primary-color);
}

.loading-spinner {
  display: inline-block;
  width: 16px;
  height: 16px;
  border: 2px solid var(--primary-dim);
  border-top-color: var(--primary-color);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.error-banner {
  background: var(--error-dim);
  border: 1px solid var(--error-color);
  border-radius: 6px;
  padding: 8px 12px;
  font-size: 12px;
  color: var(--error-color);
}

.content {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.file-list-wrapper {
  flex: 1;
  min-height: 0;
}
</style>
