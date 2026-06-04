# Sample Rate & Bit Rate UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add user-adjustable sample rate and bit rate dropdown selectors to the MP3 conversion tool, replacing hardcoded 44100 Hz / 192 kbps values.

**Architecture:** New `OutputSettings.vue` component sits above `ActionBar`, exposes settings via `v-model`. Settings flow through `App.vue` → `useConverter` → Tauri invoke → Rust `converter.rs` where hardcoded values are replaced with parameters.

**Tech Stack:** Vue 3 (Composition API), TypeScript, Tauri 2, Rust, mp3lame-encoder, Rubato

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src/types.ts` | Modify | Add `OutputSettings` interface |
| `src/components/OutputSettings.vue` | Create | Sample rate + bit rate dropdown selectors |
| `src/components/FileItem.vue` | Modify | Dynamic bit rate display in target format column |
| `src/App.vue` | Modify | Integrate OutputSettings, wire data flow |
| `src/composables/useConverter.ts` | Modify | `startConversion` accepts sample rate + bit rate |
| `src-tauri/src/commands.rs` | Modify | `start_conversion` gains `sample_rate`, `bit_rate` params |
| `src-tauri/src/converter.rs` | Modify | Parameterize resample target and LAME encoding |

---

### Task 1: Add `OutputSettings` type to `types.ts`

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: Add `OutputSettings` interface**

Add after the `FileEntry` interface (line 34):

```typescript
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
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /Users/chon/Source/github.com/outman/FrogVoice && pnpm run build 2>&1 | head -20`
Expected: Build succeeds (no errors from types.ts)

- [ ] **Step 3: Commit**

```bash
git add src/types.ts
git commit -m "feat: add OutputSettings type with sample rate and bit rate options"
```

---

### Task 2: Create `OutputSettings.vue` component

**Files:**
- Create: `src/components/OutputSettings.vue`

- [ ] **Step 1: Create the component**

Create `src/components/OutputSettings.vue` with this content:

```vue
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
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /Users/chon/Source/github.com/outman/FrogVoice && pnpm run build 2>&1 | head -20`
Expected: Build succeeds

- [ ] **Step 3: Commit**

```bash
git add src/components/OutputSettings.vue
git commit -m "feat: create OutputSettings component with sample rate and bit rate dropdowns"
```

---

### Task 3: Update `useConverter.ts` to pass settings to backend

**Files:**
- Modify: `src/composables/useConverter.ts`

- [ ] **Step 1: Update `startConversion` signature and invoke call**

In `src/composables/useConverter.ts`, change the `startConversion` function (lines 68-87).

Replace:
```typescript
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
```

With:
```typescript
  async function startConversion(outputDir: string, sampleRate: number, bitRate: number) {
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
      await invoke('start_conversion', { files: audioFiles, outputDir, sampleRate, bitRate });
    } catch (e) {
      error.value = String(e);
      isConverting.value = false;
    }
  }
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /Users/chon/Source/github.com/outman/FrogVoice && pnpm run build 2>&1 | head -20`
Expected: Build succeeds (TypeScript won't error on unused params yet since callers haven't been updated)

- [ ] **Step 3: Commit**

```bash
git add src/composables/useConverter.ts
git commit -m "feat: pass sample rate and bit rate params to start_conversion command"
```

---

### Task 4: Update `App.vue` to integrate OutputSettings

**Files:**
- Modify: `src/App.vue`

- [ ] **Step 1: Add import and state**

In the `<script setup>` block, add after the existing imports (line 6):

```typescript
import OutputSettings from './components/OutputSettings.vue';
import { DEFAULT_OUTPUT_SETTINGS, type OutputSettings } from './types';
```

Add after line 9 (`const outputDir = ref('');`):

```typescript
const outputSettings = ref<OutputSettings>({ ...DEFAULT_OUTPUT_SETTINGS });
```

- [ ] **Step 2: Update `handleStart` to pass settings**

Replace the `handleStart` function (lines 35-38):

```typescript
async function handleStart() {
  if (!outputDir.value) return;
  await startConversion(outputDir.value, outputSettings.value.sampleRate, outputSettings.value.bitRate);
}
```

- [ ] **Step 3: Add `OutputSettings` component to template**

In the `<template>`, inside `.content` div (after line 78 `<FileList ... />` and before `<ActionBar ... />`), insert:

```html
      <OutputSettings v-model:settings="outputSettings" :disabled="isLoading" />
```

- [ ] **Step 4: Verify it compiles**

Run: `cd /Users/chon/Source/github.com/outman/FrogVoice && pnpm run build 2>&1 | head -20`
Expected: Build succeeds

- [ ] **Step 5: Commit**

```bash
git add src/App.vue
git commit -m "feat: integrate OutputSettings into App with data binding and disabled state"
```

---

### Task 5: Update `FileItem.vue` for dynamic bit rate display

**Files:**
- Modify: `src/components/FileItem.vue`

- [ ] **Step 1: Add `bitRate` prop and update template**

Replace line 5:

```typescript
defineProps<{ entry: FileEntry }>();
```

With:
```typescript
defineProps<{ entry: FileEntry; bitRate: number }>();
```

Replace line 16:

```html
    <div class="file-format">192kbps → MP3</div>
```

With:
```html
    <div class="file-format">{{ bitRate }}kbps → MP3</div>
```

- [ ] **Step 2: Update `FileList.vue` to pass `bitRate` through**

In `src/components/FileList.vue`, we need to check if it passes props through to `FileItem`. Read the file first to confirm its structure, then add a `bitRate` prop to `FileList` and pass it to each `FileItem`.

After reading, the likely change is:
- Add `bitRate` prop to `FileList` props
- Pass `:bit-rate="bitRate"` to each `<FileItem>`

- [ ] **Step 3: Update `App.vue` to pass `bitRate` to `FileList`**

In `App.vue`, update the `<FileList>` usage:

```html
<FileList :files="files" :bit-rate="outputSettings.bitRate" class="file-list-wrapper" />
```

- [ ] **Step 4: Verify it compiles**

Run: `cd /Users/chon/Source/github.com/outman/FrogVoice && pnpm run build 2>&1 | head -20`
Expected: Build succeeds

- [ ] **Step 5: Commit**

```bash
git add src/components/FileItem.vue src/components/FileList.vue src/App.vue
git commit -m "feat: show dynamic bit rate in file item target format column"
```

---

### Task 6: Update Rust `commands.rs` to accept new parameters

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Add `sample_rate` and `bit_rate` params to `start_conversion`**

Replace the `start_conversion` function (lines 30-54):

```rust
#[tauri::command]
pub fn start_conversion(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    files: Vec<AudioFile>,
    output_dir: String,
    sample_rate: u32,
    bit_rate: u32,
) -> Result<(), String> {
    if state.converting.load(Ordering::Relaxed) {
        return Err("已有转换任务正在运行".to_string());
    }

    // Reset cancel flag
    state.cancel_flag.store(false, Ordering::Relaxed);
    state.converting.store(true, Ordering::Relaxed);

    let cancel_flag = state.cancel_flag.clone();
    let converting = state.converting.clone();
    let app_handle = app.clone();

    std::thread::spawn(move || {
        converter::convert_files(&app_handle, &files, &output_dir, &cancel_flag, sample_rate, bit_rate);
        converting.store(false, Ordering::Relaxed);
    });

    Ok(())
}
```

- [ ] **Step 2: Verify Rust compiles**

Run: `cd /Users/chon/Source/github.com/outman/FrogVoice/src-tauri && cargo check 2>&1 | tail -5`
Expected: Will fail — `convert_files` signature doesn't match yet. That's fine, fixed in Task 7.

- [ ] **Step 3: Commit (after Task 7 compiles)**

---

### Task 7: Update Rust `converter.rs` to use parameterized values

**Files:**
- Modify: `src-tauri/src/converter.rs`

- [ ] **Step 1: Update `convert_files` signature**

Replace the `convert_files` function signature (line 68):

```rust
pub fn convert_files(
    app: &AppHandle,
    files: &[AudioFile],
    output_dir: &str,
    cancel_flag: &AtomicBool,
) -> ConvertDoneEvent {
```

With:
```rust
pub fn convert_files(
    app: &AppHandle,
    files: &[AudioFile],
    output_dir: &str,
    cancel_flag: &AtomicBool,
    sample_rate: u32,
    bit_rate: u32,
) -> ConvertDoneEvent {
```

- [ ] **Step 2: Update the `convert_single_file` call inside `convert_files`**

Replace line 112:

```rust
        match convert_single_file(app, &file.path, &output_path, cancel_flag) {
```

With:
```rust
        match convert_single_file(app, &file.path, &output_path, cancel_flag, sample_rate, bit_rate) {
```

- [ ] **Step 3: Update `convert_single_file` signature and body**

Replace the `convert_single_file` function (lines 146-203):

```rust
/// Convert a single audio file to MP3.
fn convert_single_file(
    app: &AppHandle,
    input_path: &str,
    output_path: &str,
    cancel_flag: &AtomicBool,
    sample_rate: u32,
    bit_rate: u32,
) -> Result<u64, String> {
    let start = Instant::now();
    let input_path_owned = input_path.to_string();

    // Phase 1: Decode (0% → 80% of progress)
    let pcm = decoder::decode_to_pcm(input_path, &|progress| {
        let _ = app.emit(
            "convert-progress",
            ConvertProgressEvent {
                file_path: input_path_owned.clone(),
                progress,
            },
        );
    })?;

    if cancel_flag.load(Ordering::Relaxed) {
        return Err("转换已取消".to_string());
    }
    if start.elapsed() > Duration::from_secs(CONVERT_TIMEOUT_SECS) {
        return Err("转换超时（10分钟）".to_string());
    }

    // Phase 2: Ensure stereo (mono → duplicate channels)
    let stereo_pcm = ensure_stereo(pcm);

    // Phase 3: Resample to target sample rate if needed
    let resampled = resample_if_needed(stereo_pcm, sample_rate)?;

    // Phase 4: Interleave planar to interleaved
    let interleaved = interleave(&resampled);

    if cancel_flag.load(Ordering::Relaxed) {
        return Err("转换已取消".to_string());
    }

    // Phase 5: Emit 80% progress
    let _ = app.emit(
        "convert-progress",
        ConvertProgressEvent {
            file_path: input_path.to_string(),
            progress: 0.8,
        },
    );

    // Phase 6: Encode to MP3
    encode_mp3(&interleaved, 2, sample_rate, bit_rate, output_path)?;

    let output_size = std::fs::metadata(output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(output_size)
}
```

- [ ] **Step 4: Update `resample_if_needed` to accept target sample rate**

Replace the `resample_if_needed` function signature (line 224):

```rust
fn resample_if_needed(pcm: decoder::PcmData) -> Result<decoder::PcmData, String> {
```

With:
```rust
fn resample_if_needed(pcm: decoder::PcmData, target_sample_rate: u32) -> Result<decoder::PcmData, String> {
```

Then inside the function body, replace every occurrence of the hardcoded `44100` with `target_sample_rate`:

Line 225 — change `if pcm.sample_rate == 44100` to:
```rust
    if pcm.sample_rate == target_sample_rate {
```

Lines 230-236 — change `sample_rate: 44100` to:
```rust
        return Ok(decoder::PcmData {
            samples: pcm.samples,
            sample_rate: target_sample_rate,
            channels: pcm.channels,
            duration_secs: pcm.duration_secs,
        });
```

Line 250 — change `let ratio = 44100.0 /` to:
```rust
    let ratio = target_sample_rate as f64 / pcm.sample_rate as f64;
```

Lines 280-281 — change the partial chunk truncation formula:
```rust
            ((actual_chunk as f64 * target_sample_rate as f64 / pcm.sample_rate as f64).round() as usize)
```

Lines 293-294 — change `output[0].len() as f64 / 44100.0` to:
```rust
        output[0].len() as f64 / target_sample_rate as f64
```

Lines 299-300 — change `sample_rate: 44100` to:
```rust
        sample_rate: target_sample_rate,
```

- [ ] **Step 5: Update `encode_mp3` to accept and use `bit_rate` parameter**

Replace the `encode_mp3` signature (line 323):

```rust
fn encode_mp3(
    pcm: &[f32],
    num_channels: usize,
    sample_rate: u32,
    output_path: &str,
) -> Result<(), String> {
```

With:
```rust
fn encode_mp3(
    pcm: &[f32],
    num_channels: usize,
    sample_rate: u32,
    bit_rate: u32,
    output_path: &str,
) -> Result<(), String> {
```

Replace the hardcoded `Kbps192` on line 337:

```rust
        .with_brate(mp3lame_encoder::Bitrate::Kbps192)
```

With:
```rust
        .with_brate(match bit_rate {
            128 => mp3lame_encoder::Bitrate::Kbps128,
            192 => mp3lame_encoder::Bitrate::Kbps192,
            256 => mp3lame_encoder::Bitrate::Kbps256,
            320 => mp3lame_encoder::Bitrate::Kbps320,
            _ => mp3lame_encoder::Bitrate::Kbps192,
        })
```

- [ ] **Step 6: Verify the full project compiles**

Run: `cd /Users/chon/Source/github.com/outman/FrogVoice && pnpm run build && cd src-tauri && cargo check 2>&1 | tail -5`
Expected: Both frontend and backend compile successfully

- [ ] **Step 7: Commit Tasks 6 + 7 together**

```bash
git add src-tauri/src/commands.rs src-tauri/src/converter.rs
git commit -m "feat: parameterize sample rate and bit rate in Rust conversion pipeline"
```

---

### Task 8: Manual verification

- [ ] **Step 1: Run the app**

Run: `cd /Users/chon/Source/github.com/outman/FrogVoice && pnpm tauri dev`

- [ ] **Step 2: Verify UI**

- [ ] Two dropdowns appear above the ActionBar: 采样率 and 比特率
- [ ] Defaults show "44100 Hz" and "192 kbps"
- [ ] File items show "192kbps → MP3" matching selected bit rate
- [ ] Changing bit rate updates file item display in real time
- [ ] Dropdowns are disabled during conversion

- [ ] **Step 3: Test conversion with non-default settings**

- [ ] Change sample rate to 48000 Hz, bit rate to 320 kbps
- [ ] Run a conversion
- [ ] Verify the output MP3 file has the correct properties (e.g., using `ffprobe` or file size difference)

- [ ] **Step 4: Final commit if any fixes were needed**

```bash
git add -A
git commit -m "fix: polish sample rate and bit rate UI"
```
