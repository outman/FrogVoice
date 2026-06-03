import { ref, reactive } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  AudioFile,
  FileEntry,
  ConvertProgressEvent,
  ConvertStatusEvent,
  ConvertDoneEvent,
} from '../types';

export function useConverter() {
  const files = reactive<FileEntry[]>([]);
  const isConverting = ref(false);
  const isScanning = ref(false);
  const doneSummary = ref<ConvertDoneEvent | null>(null);
  const error = ref('');

  let unlisteners: UnlistenFn[] = [];

  async function startListening() {
    stopListening();

    const u1 = await listen<ConvertProgressEvent>('convert-progress', (event) => {
      const entry = files.find((f) => f.file.path === event.payload.file_path);
      if (entry) {
        entry.status = { type: 'Converting', progress: event.payload.progress };
      }
    });

    const u2 = await listen<ConvertStatusEvent>('convert-status', (event) => {
      const entry = files.find((f) => f.file.path === event.payload.file_path);
      if (entry) {
        entry.status = event.payload.status;
      }
    });

    const u3 = await listen<ConvertDoneEvent>('convert-done', (event) => {
      doneSummary.value = event.payload;
      isConverting.value = false;
    });

    unlisteners = [u1, u2, u3];
  }

  function stopListening() {
    unlisteners.forEach((u) => u());
    unlisteners = [];
  }

  async function scanFiles(inputDir: string) {
    isScanning.value = true;
    error.value = '';
    doneSummary.value = null;
    try {
      const result = await invoke<AudioFile[]>('scan_audio_files', { inputDir });
      files.splice(0, files.length);
      for (const file of result) {
        files.push({ file, status: { type: 'Pending' } });
      }
    } catch (e) {
      error.value = String(e);
    } finally {
      isScanning.value = false;
    }
  }

  async function startConversion(outputDir: string) {
    if (isConverting.value || files.length === 0) return;

    // Reset all statuses to Pending
    for (const entry of files) {
      entry.status = { type: 'Pending' };
    }
    doneSummary.value = null;
    isConverting.value = true;

    await startListening();

    try {
      const audioFiles = files.map((f) => f.file);
      await invoke('start_conversion', { files: audioFiles, outputDir });
    } catch (e) {
      error.value = String(e);
      isConverting.value = false;
    }
  }

  async function openFolder(path: string) {
    try {
      await invoke('open_folder', { path });
    } catch (e) {
      error.value = String(e);
    }
  }

  function reset() {
    stopListening();
    files.splice(0, files.length);
    isConverting.value = false;
    doneSummary.value = null;
    error.value = '';
  }

  return {
    files,
    isConverting,
    isScanning,
    doneSummary,
    error,
    scanFiles,
    startConversion,
    openFolder,
    reset,
  };
}
