export interface AudioFile {
  name: string;
  path: string;
  size: number;
  duration_secs: number | null;
  extension: string;
}

export type FileConvertStatus =
  | { type: 'Pending' }
  | { type: 'Converting'; progress: number }
  | { type: 'Completed'; output_size: number }
  | { type: 'Failed'; error: string };

export interface ConvertProgressEvent {
  file_path: string;
  progress: number;
}

export interface ConvertStatusEvent {
  file_path: string;
  status: FileConvertStatus;
}

export interface ConvertDoneEvent {
  total: number;
  success: number;
  failed: number;
}

export interface FileEntry {
  file: AudioFile;
  status: FileConvertStatus;
}

export interface OutputSettings {
  sampleRate: number;   // 44100 | 48000
  bitRate: number;      // 128 | 192 | 256 | 320
}

export const SAMPLE_RATE_OPTIONS = [
  { label: '44100 Hz', value: 44100 },
  { label: '48000 Hz', value: 48000 },
] as const;

export const BIT_RATE_OPTIONS = [
  { label: '128 kbps', value: 128 },
  { label: '192 kbps', value: 192 },
  { label: '256 kbps', value: 256 },
  { label: '320 kbps', value: 320 },
] as const;

export const DEFAULT_OUTPUT_SETTINGS: OutputSettings = {
  sampleRate: 44100,
  bitRate: 192,
};

export function formatDuration(secs: number | null): string {
  if (secs === null || secs === undefined) return '--:--';
  const minutes = Math.floor(secs / 60);
  const seconds = Math.floor(secs % 60);
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
}

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function getStatusText(status: FileConvertStatus): string {
  switch (status.type) {
    case 'Pending': return '⏳ 等待中';
    case 'Converting': return `🔄 ${Math.round(status.progress * 100)}%`;
    case 'Completed': return '✅ 已完成';
    case 'Failed': return `❌ 失败`;
  }
}

export function getStatusClass(status: FileConvertStatus): string {
  switch (status.type) {
    case 'Pending': return 'status-pending';
    case 'Converting': return 'status-converting';
    case 'Completed': return 'status-completed';
    case 'Failed': return 'status-failed';
  }
}
