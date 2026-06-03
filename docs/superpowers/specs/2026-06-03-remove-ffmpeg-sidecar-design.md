# Remove FFmpeg Sidecar — Pure Rust Audio Conversion

**Date:** 2026-06-03
**Status:** Approved
**Scope:** Replace ffmpeg sidecar binary with Rust-native audio decoding/encoding

## Background

FrogVoice currently bundles ~250MB of ffmpeg binaries (macOS arm64, macOS x86_64, Windows x86_64) as Tauri sidecars. This causes:

- Large download size (~77-97MB per platform)
- macOS Gatekeeper/code-signing complications for the sidecar binary
- Complex build/packaging scripts to manage sidecar binaries
- Shell permission prompts for subprocess spawning

The app only uses two ffmpeg operations:
1. **Duration probe**: `ffmpeg -i <file>` — parse Duration from stderr
2. **MP3 conversion**: `ffmpeg -y -i <input> -vn -acodec libmp3lame -ar 44100 -ac 2 -b:a 192k <output>`

## Decision

**Approach B (Hybrid):** symphonia (pure Rust) for decoding + mp3lame-encoder (LAME C bindings) for MP3 encoding.

- Decoding: 100% pure Rust via symphonia — no C dependencies for input
- Encoding: LAME via `mp3lame-encoder` — the only C library, small (~1MB source), battle-tested MP3 quality
- Resampling: rubato (pure Rust) for sample rate conversion to 44100Hz

Rationale: LAME produces significantly higher quality MP3 output than the pure Rust alternative (shine-rs). Compiling one small C library is far simpler than building the full ffmpeg toolchain.

## Supported Audio Formats

### Input (decoding via symphonia)

| Format | Extensions | Symphonia Feature |
|--------|-----------|-------------------|
| WAV/PCM | `.wav` | `wav`, `pcm` |
| MP3 | `.mp3` | `mpa` |
| FLAC | `.flac` | `flac` |
| AAC | `.aac` | `aac` |
| M4A/MP4 | `.m4a`, `.mp4`, `.m4v` | `isomp4`, `aac`, `alac` |
| OGG/Vorbis | `.ogg`, `.oga` | `ogg` |
| AIFF | `.aiff`, `.aif` | `aiff` |

### Output

- MP3 (CBR 192kbps, 44100Hz, stereo) via LAME encoder

### Not supported (removed from original list)

OPUS, WMA, APE, DSF, AC3, AMR, AU/SND, MIDI, WV, TTA — rare formats, can be added later via individual crates if needed.

## Architecture

### Module Structure

```
src-tauri/src/
├── lib.rs              # Remove tauri_plugin_shell registration
├── models.rs           # Unchanged (AudioFile, events, etc.)
├── commands.rs         # Remove shell dependency, call converter API
├── converter.rs        # Rewrite: decode → resample → encode pipeline
├── audio_meta.rs       # Simplify: keep format_duration(), remove ffmpeg parsing
└── decoder.rs          # NEW: symphonia wrapper (probe duration, decode to PCM)
```

### Data Flow

```
Input File
  │
  ├─ Scan Phase ──────────────────────────────────────────
  │  decoder::probe_duration(path) → Option<f64>
  │    - symphonia FormatReader opens file
  │    - Read track metadata for duration
  │    - Fallback: estimate from bitrate + file size
  │
  ├─ Convert Phase ───────────────────────────────────────
  │  decoder::decode_to_pcm(path) → PcmData
  │    - symphonia decodes all frames to f32 samples
  │    - Select audio track (skip video in MP4)
  │
  │  resample(pcm, target_sr=44100, target_channels=2)
  │    - rubato resampler converts sample rate
  │    - Mono → stereo channel duplication
  │
  │  encode_mp3(pcm_44100_stereo, output_path) → u64
  │    - mp3lame-encoder: CBR 192kbps, 44100Hz, stereo
  │    - Write frames to output file
  │    - Return output file size
  │
  ▼
Output .mp3 File
```

### PcmData Structure

```rust
struct PcmData {
    samples: Vec<f32>,      // interleaved f32 samples
    sample_rate: u32,
    channels: usize,
    duration_secs: f64,
}
```

### Progress Reporting

Replace ffmpeg stderr parsing with direct computation:

- **Decode phase** (0-50% of progress): decoded_frames / estimated_total_frames
- **Encode phase** (50-100%): encoded_bytes / estimated_output_size (192000/8 * duration)

Events emitted remain identical:
- `convert-progress` with `{ file_path, progress: 0.0-1.0 }`
- `convert-status` with per-file completion/failure
- `convert-done` with summary

Frontend requires **zero changes**.

## Dependency Changes

### Remove

```toml
tauri-plugin-shell = "2"
```

### Add

```toml
symphonia = { version = "0.5", features = ["aac", "flac", "mp3", "ogg", "wav", "aiff", "alac", "isomp4", "mpa"] }
mp3lame-encoder = "1"
rubato = "0.15"
```

### Config Changes

**tauri.conf.json:**
- Remove `"externalBin": ["binaries/ffmpeg"]`

**capabilities/default.json:**
- Remove `"shell:allow-spawn"`, `"shell:allow-kill"`

### Files to Delete

- `src-tauri/binaries/ffmpeg-*` (all platform binaries)
- `scripts/download-ffmpeg.sh`

### Files to Simplify

- `scripts/build.sh` — remove ffmpeg binary validation
- `scripts/package.sh` — remove sidecar packaging logic

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Unsupported format | Return "不支持的音频格式: {ext}" error for that file |
| Corrupted file | Return "解码失败: {reason}" error, skip to next file |
| No audio track in MP4 | Return "未找到音频轨道" error |
| Cannot determine duration | Proceed with duration = None, progress shows indeterminate |
| LAME encoding error | Return "MP3 编码失败: {reason}" error |

Errors are per-file; batch conversion continues with remaining files.

## What Does NOT Change

- **Frontend (Vue/TypeScript)**: No changes — same invoke/listen API
- **models.rs**: Same data types (AudioFile, events)
- **UI layout and styling**: Unchanged
- **Cancel mechanism**: Same AtomicBool pattern

## Distribution Impact

| Metric | Before | After |
|--------|--------|-------|
| Sidecar binaries | 3 files, ~250MB total | None |
| App binary size increase | — | ~2-3MB (statically linked codecs) |
| Total download size | ~350MB+ | ~100MB (app only) |
| Code signing complexity | Sidecar needs separate signing | Single binary |
| Cross-compilation | Download 3 platform ffmpeg builds | Standard cargo cross-compile |
