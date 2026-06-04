<script setup lang="ts">
import type { OutputSettings } from '../types';
import { SAMPLE_RATE_OPTIONS, BIT_RATE_OPTIONS } from '../types';

const settings = defineModel<OutputSettings>('settings', { required: true });

defineProps<{
  disabled?: boolean;
}>();
</script>

<template>
  <div class="output-settings">
    <div class="setting-item">
      <label class="setting-label">采样率</label>
      <select
        v-model="settings.sampleRate"
        class="setting-select"
        :disabled="disabled"
      >
        <option
          v-for="opt in SAMPLE_RATE_OPTIONS"
          :key="opt.value"
          :value="opt.value"
        >
          {{ opt.label }}
        </option>
      </select>
    </div>
    <div class="setting-item">
      <label class="setting-label">比特率</label>
      <select
        v-model="settings.bitRate"
        class="setting-select"
        :disabled="disabled"
      >
        <option
          v-for="opt in BIT_RATE_OPTIONS"
          :key="opt.value"
          :value="opt.value"
        >
          {{ opt.label }}
        </option>
      </select>
    </div>
  </div>
</template>

<style scoped>
.output-settings {
  display: flex;
  gap: 16px;
  padding: 8px 0;
  margin-top: 12px;
  border-top: 1px solid var(--border-color);
}

.setting-item {
  display: flex;
  align-items: center;
  gap: 8px;
}

.setting-label {
  font-size: 12px;
  color: var(--text-secondary);
  white-space: nowrap;
}

.setting-select {
  background: var(--surface-color);
  color: var(--text-primary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 4px 8px;
  font-size: 12px;
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
</style>
