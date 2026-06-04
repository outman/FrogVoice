# Sample Rate & Bit Rate UI Design

## Overview

Add user-adjustable sample rate and bit rate parameters to the MP3 conversion tool. Currently these are hardcoded as 44100 Hz / 192 kbps CBR. Users will be able to select from preset values before starting conversion.

## UI Layout

**Position:** Horizontal row of two dropdown selectors, placed directly above the ActionBar at the bottom of the window.

```
┌─────────────────────────────────────────────────────┐
│                                                     │
│  (file list area)                                   │
│                                                     │
├─────────────────────────────────────────────────────┤
│  采样率: [44100 Hz ▾]     比特率: [192 kbps ▾]      │
├─────────────────────────────────────────────────────┤
│           [开始转换]  [打开输出目录]  [重置]          │
└─────────────────────────────────────────────────────┘
```

### New Component: `OutputSettings.vue`

- Two dropdown selectors side by side
- **Sample rate options:** 44100 Hz, 48000 Hz
- **Bit rate options:** 128 kbps, 192 kbps, 256 kbps, 320 kbps
- **Defaults:** 44100 Hz / 192 kbps (matches current hardcoded values)
- Exposes selected values via `v-model:settings`
- Accepts `disabled` prop — controls are disabled during active conversion
- Styled to match the existing dark theme (`--surface-color`, `--primary-color`, etc.)

### Existing Component Changes

- **`FileItem.vue`:** Target format column (currently hardcoded `192kbps -> MP3`) becomes dynamic, reflecting the currently selected bit rate
- **`App.vue`:** Holds `ref<OutputSettings>`, binds to `OutputSettings` via `v-model:settings`, passes `isConverting` as `disabled`, forwards settings to `useConverter.startConversion()`

## Data Flow

```
OutputSettings.vue
    ↕ v-model:settings
App.vue (ref<OutputSettings> { sampleRate, bitRate })
    ↓ arguments
useConverter.ts → startConversion(outputDir, sampleRate, bitRate)
    ↓ Tauri invoke
commands.rs → start_conversion(files, output_dir, sample_rate, bit_rate)
    ↓ pass through
converter.rs → convert_files(..., sample_rate, bit_rate)
    ↓ use in pipeline
converter.rs → convert_single_file(..., sample_rate, bit_rate)
    ↓ resample to sample_rate, encode with bit_rate
```

Parameters are read only at the moment the user clicks "Start Conversion". Changing settings during an active conversion has no effect on the running batch.

## Type Definitions

### TypeScript (`types.ts`)

```typescript
export interface OutputSettings {
  sampleRate: number   // 44100 | 48000
  bitRate: number      // 128 | 192 | 256 | 320
}
```

### Rust

No model changes needed. Sample rate and bit rate are passed as `u32` parameters to the `start_conversion` command.

## Backend Changes

### `commands.rs`

`start_conversion` gains two new parameters: `sample_rate: u32`, `bit_rate: u32`, forwarded to `converter::convert_files`.

### `converter.rs`

- `convert_files()` and `convert_single_file()` accept `sample_rate: u32` and `bit_rate: u32`
- **Resample stage:** use the passed `sample_rate` instead of hardcoded `44100`
- `encode_mp3()` gains `sample_rate` and `bit_rate` parameters:
  - `sample_rate` passed to LAME encoder configuration
  - `bit_rate` mapped via `match` to `mp3lame_encoder::Bitrate` enum variants

### Bitrate mapping (Rust)

```rust
match bit_rate {
    128 => mp3lame_encoder::Bitrate::Kbps128,
    192 => mp3lame_encoder::Bitrate::Kbps192,
    256 => mp3lame_encoder::Bitrate::Kbps256,
    320 => mp3lame_encoder::Bitrate::Kbps320,
    _ => mp3lame_encoder::Bitrate::Kbps192, // fallback
}
```

## Edge Cases & Interaction Details

- Dropdown options display friendly labels (e.g., "44100 Hz") but send numeric values (e.g., `44100`) to the backend
- `FileItem.vue` target format updates reactively as user changes bit rate
- After conversion completes, changing parameters and clicking "Start Conversion" again uses the new parameters
- If backend receives an unsupported value, it returns an error message displayed via the existing error banner
- No input validation needed on the frontend — dropdown selectors only allow preset values

## Out of Scope

- ❌ Persisting user's last chosen settings across app restarts
- ❌ Per-file parameter overrides (global setting only)
- ❌ VBR (variable bit rate) support — keeping CBR only
- ❌ Additional sample rates beyond 44100/48000 or bit rates beyond the 4 presets

## Files Changed

| File | Change |
|------|--------|
| `src/components/OutputSettings.vue` | **New** — sample rate + bit rate dropdown selectors |
| `src/App.vue` | Integrate OutputSettings, pass params and disabled state |
| `src/components/FileItem.vue` | Dynamic bit rate display in target format column |
| `src/types.ts` | Add `OutputSettings` interface |
| `src/composables/useConverter.ts` | `startConversion` accepts sample rate and bit rate |
| `src-tauri/src/commands.rs` | `start_conversion` gains `sample_rate`, `bit_rate` params |
| `src-tauri/src/converter.rs` | Parameterize resample target and LAME encoding settings |
