# Remove FFmpeg Sidecar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace ffmpeg sidecar binary with Rust-native audio decoding (symphonia) and MP3 encoding (mp3lame-encoder).

**Architecture:** Decode audio files using symphonia (pure Rust), resample to 44100Hz stereo using rubato if needed, encode to MP3 using mp3lame-encoder (LAME bindings). All processing happens in-process — no subprocess spawning.

**Tech Stack:** symphonia 0.5, mp3lame-encoder 1, rubato 0.15

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src-tauri/Cargo.toml` | Modify | Add symphonia/mp3lame-encoder/rubato, remove tauri-plugin-shell |
| `src-tauri/src/decoder.rs` | Create | Symphonia wrapper: probe duration, decode to planar f32 PCM |
| `src-tauri/src/converter.rs` | Rewrite | New pipeline: decode → resample → interleave → encode MP3 |
| `src-tauri/src/audio_meta.rs` | Simplify | Keep format_duration, remove ffmpeg stderr parsing |
| `src-tauri/src/commands.rs` | Modify | Remove shell dependency |
| `src-tauri/src/lib.rs` | Modify | Remove tauri_plugin_shell registration |
| `src-tauri/tauri.conf.json` | Modify | Remove externalBin |
| `src-tauri/capabilities/default.json` | Modify | Remove shell permissions |
| `scripts/build.sh` | Modify | Remove ffmpeg validation |
| `scripts/package.sh` | Rewrite | Simplify without sidecar |
| `scripts/download-ffmpeg.sh` | Delete | No longer needed |
| `src-tauri/binaries/ffmpeg-*` | Delete | No longer needed |
| `package.json` | Modify | Remove @tauri-apps/plugin-shell |

---

### Task 1: Update Cargo.toml dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Replace tauri-plugin-shell with new audio dependencies**

Replace the full `[dependencies]` section:

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["time"] }
symphonia = { version = "0.5", features = ["aac", "flac", "mp3", "ogg", "wav", "aiff", "alac", "isomp4", "mpa"] }
mp3lame-encoder = "1"
rubato = "0.15"
```

Key changes:
- Removed `tauri-plugin-shell = "2"`
- Added `symphonia` with codec features for WAV/MP3/FLAC/AAC/OGG/AIFF/ALAC/MP4
- Added `mp3lame-encoder` for MP3 encoding via LAME
- Added `rubato` for audio resampling

- [ ] **Step 2: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore: replace tauri-plugin-shell with symphonia/mp3lame-encoder/rubato"
```

---

### Task 2: Create decoder.rs — Symphonia audio decoder

**Files:**
- Create: `src-tauri/src/decoder.rs`

- [ ] **Step 1: Write the decoder module**

```rust
use std::fs::File;
use std::path::Path;

use symphonia::core::audio::{SampleBuffer, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Decoded PCM audio data in planar format (one Vec per channel).
pub struct PcmData {
    /// Planar f32 samples — samples[0] is channel 0, samples[1] is channel 1, etc.
    pub samples: Vec<Vec<f32>>,
    pub sample_rate: u32,
    pub channels: usize,
    pub duration_secs: f64,
}

/// Open an audio file with symphonia and return the FormatReader + selected track info.
fn open_audio(path: &str) -> Result<(Box<dyn symphonia::core::formats::FormatReader>, symphonia::core::formats::Track), String> {
    let file = File::open(path).map_err(|e| format!("无法打开文件: {}", e))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let format_opts = FormatOptions {
        enable_gapless: true,
        ..Default::default()
    };
    let metadata_opts = MetadataOptions::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| format!("无法识别音频格式: {}", e))?;

    let format_reader = probed.format;

    // Find first audio track (has sample_rate — video tracks don't)
    let track = format_reader
        .tracks()
        .iter()
        .find(|t| t.codec_params.sample_rate.is_some())
        .or_else(|| format_reader.default_track())
        .ok_or("未找到音频轨道")?
        .clone();

    Ok((format_reader, track))
}

/// Extract duration from track codec parameters.
fn duration_from_params(params: &symphonia::core::codecs::CodecParameters) -> Option<f64> {
    let tb = params.time_base?;
    let n_frames = params.n_frames?;
    let total_ts = params.start_ts + n_frames;
    let time = tb.calc_time(total_ts);
    Some(time.seconds as f64 + time.frac)
}

/// Probe the duration of an audio file using symphonia metadata.
/// Returns None if duration cannot be determined.
pub fn probe_duration(path: &str) -> Option<f64> {
    let (mut format_reader, track) = open_audio(path).ok()?;
    let params = &track.codec_params;

    // Try to get duration from track metadata first
    if let Some(dur) = duration_from_params(params) {
        return Some(dur);
    }

    // If metadata doesn't have duration, try seeking to end
    // (not all formats support this, so we just return None)
    drop(format_reader);
    None
}

/// Decode an audio file to planar f32 PCM data.
///
/// `progress_callback` is called with values 0.0..1.0 during decode.
pub fn decode_to_pcm<F: Fn(f64)>(path: &str, progress_callback: F) -> Result<PcmData, String> {
    let (mut format_reader, track) = open_audio(path)?;
    let params = &track.codec_params;

    let track_id = track.id;
    let sample_rate = params.sample_rate.ok_or("无法获取采样率")?;
    let channels = params.channels.ok_or("无法获取声道数")?;
    let num_channels = channels.count();

    let duration_secs = duration_from_params(params).unwrap_or(0.0);

    // Create decoder
    let decode_opts = DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(params, &decode_opts)
        .map_err(|e| format!("不支持该音频编码: {}", e))?;

    // Calculate total frames for progress reporting
    let total_frames = params.n_frames.unwrap_or(0) as f64;
    let mut decoded_frames: f64 = 0.0;

    // Decode all packets into planar f32 buffers
    let mut all_samples: Vec<Vec<f32>> = vec![Vec::new(); num_channels];

    loop {
        let packet = match format_reader.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => return Err(format!("解码错误: {}", e)),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let packet_frames = packet.dur() as u64;

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                let capacity = decoded.capacity() as u64;
                let mut sample_buf = SampleBuffer::<f32>::new(capacity, spec);
                sample_buf.copy_interleaved_ref(decoded);

                let interleaved = sample_buf.samples();
                // De-interleave: interleaved is [L0, R0, L1, R1, ...]
                for (i, sample) in interleaved.iter().enumerate() {
                    all_samples[i % num_channels].push(*sample);
                }

                decoded_frames += packet_frames as f64;
                if total_frames > 0.0 {
                    let progress = (decoded_frames / total_frames).min(1.0);
                    progress_callback(progress * 0.8); // decode is ~80% of work
                }
            }
            Err(Error::DecodeError(_)) => continue,
            Err(e) => return Err(format!("解码错误: {}", e)),
        }
    }

    // Calculate actual duration from samples if metadata didn't provide it
    let actual_duration = if duration_secs > 0.0 {
        duration_secs
    } else if num_channels > 0 && !all_samples[0].is_empty() {
        all_samples[0].len() as f64 / sample_rate as f64
    } else {
        0.0
    };

    Ok(PcmData {
        samples: all_samples,
        sample_rate,
        channels: num_channels,
        duration_secs: actual_duration,
    })
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/decoder.rs
git commit -m "feat: add decoder module with symphonia-based audio decoding"
```

---

### Task 3: Rewrite converter.rs — New conversion pipeline

**Files:**
- Rewrite: `src-tauri/src/converter.rs`

- [ ] **Step 1: Write the new converter module**

```rust
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use tauri::{AppHandle, Emitter};

use crate::decoder;
use crate::models::*;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "wav", "mp3", "flac", "aac", "ogg", "oga", "m4a",
    "aiff", "aif", "mp4", "m4v",
];

const CONVERT_TIMEOUT_SECS: u64 = 600;

/// Check if a file has a supported audio extension
pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Get the duration of an audio file using symphonia metadata probe.
pub fn get_audio_duration(file_path: &str) -> Option<f64> {
    decoder::probe_duration(file_path)
}

/// Scan a directory for audio files
pub fn scan_directory(input_dir: &str) -> Result<Vec<AudioFile>, String> {
    let dir = Path::new(input_dir);
    if !dir.is_dir() {
        return Err(format!("路径不是有效的文件夹: {}", input_dir));
    }

    let mut files = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| format!("无法读取文件夹: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("读取目录条目失败: {}", e))?;
        let path = entry.path();

        if path.is_file() && is_audio_file(&path) {
            let metadata =
                std::fs::metadata(&path).map_err(|e| format!("无法读取文件信息: {}", e))?;
            let path_str = path.to_string_lossy().to_string();
            let duration = get_audio_duration(&path_str);

            files.push(AudioFile {
                name: path.file_name().unwrap().to_string_lossy().to_string(),
                path: path_str,
                size: metadata.len(),
                duration_secs: duration,
                extension: path.extension().unwrap().to_string_lossy().to_lowercase(),
            });
        }
    }

    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

/// Convert a list of audio files to MP3, emitting events for progress
pub fn convert_files(
    app: &AppHandle,
    files: &[AudioFile],
    output_dir: &str,
    cancel_flag: &AtomicBool,
) -> ConvertDoneEvent {
    let total = files.len();
    let mut success = 0;
    let mut failed = 0;

    if let Err(_e) = std::fs::create_dir_all(output_dir) {
        let _ = app.emit(
            "convert-done",
            ConvertDoneEvent { total, success: 0, failed: total },
        );
        return ConvertDoneEvent { total, success: 0, failed: total };
    }

    for file in files {
        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }

        let output_path = Path::new(output_dir)
            .join(Path::new(&file.name).with_extension("mp3"))
            .to_string_lossy()
            .to_string();

        let _ = app.emit(
            "convert-status",
            ConvertStatusEvent {
                file_path: file.path.clone(),
                status: FileConvertStatus::Converting { progress: 0.0 },
            },
        );

        match convert_single_file(app, &file.path, &output_path, file.duration_secs, cancel_flag) {
            Ok(output_size) => {
                success += 1;
                let _ = app.emit(
                    "convert-status",
                    ConvertStatusEvent {
                        file_path: file.path.clone(),
                        status: FileConvertStatus::Completed { output_size },
                    },
                );
            }
            Err(error) => {
                failed += 1;
                let _ = app.emit(
                    "convert-status",
                    ConvertStatusEvent {
                        file_path: file.path.clone(),
                        status: FileConvertStatus::Failed { error },
                    },
                );
            }
        }
    }

    let event = ConvertDoneEvent { total, success, failed };
    let _ = app.emit("convert-done", &event);
    event
}

/// Convert a single audio file to MP3.
fn convert_single_file(
    app: &AppHandle,
    input_path: &str,
    output_path: &str,
    duration: Option<f64>,
    cancel_flag: &AtomicBool,
) -> Result<u64, String> {
    let start = Instant::now();
    let input_path_owned = input_path.to_string();

    // Phase 1: Decode (0% → 80% of progress)
    let pcm = decoder::decode_to_pcm(input_path, &|progress| {
        if start.elapsed() > CONVERT_TIMEOUT_SECS as Duration {
            return;
        }
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

    // Phase 3: Resample to 44100Hz if needed
    let resampled = resample_if_needed(stereo_pcm)?;

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
    encode_mp3(&interleaved, 2, 44100, output_path)?;

    let output_size = std::fs::metadata(output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(output_size)
}

/// Ensure PCM data has exactly 2 channels (stereo).
/// If mono, duplicate the single channel. If >2 channels, take first 2.
fn ensure_stereo(pcm: decoder::PcmData) -> decoder::PcmData {
    let samples = match pcm.channels {
        0 => {
            // No channels — create silence
            vec![Vec::new(), Vec::new()]
        }
        1 => {
            // Mono → stereo: duplicate channel
            vec![pcm.samples[0].clone(), pcm.samples[0].clone()]
        }
        2 => pcm.samples,
        _ => {
            // Multi-channel → take first 2
            vec![pcm.samples[0].clone(), pcm.samples[1].clone()]
        }
    };

    decoder::PcmData {
        samples,
        sample_rate: pcm.sample_rate,
        channels: 2,
        duration_secs: pcm.duration_secs,
    }
}

/// Resample PCM data to 44100Hz if not already at that rate.
fn resample_if_needed(pcm: decoder::PcmData) -> Result<decoder::PcmData, String> {
    if pcm.sample_rate == 44100 {
        return Ok(pcm);
    }

    if pcm.samples.is_empty() || pcm.samples[0].is_empty() {
        return Ok(decoder::PcmData {
            samples: pcm.samples,
            sample_rate: 44100,
            channels: pcm.channels,
            duration_secs: pcm.duration_secs,
        });
    }

    let total_frames = pcm.samples[0].len();
    let num_channels = pcm.samples.len();
    let chunk_size = 4096;

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let ratio = pcm.sample_rate as f64 / 44100.0;
    let mut resampler = SincFixedIn::<f32>::new(ratio, 2.0, params, chunk_size, num_channels)
        .map_err(|e| format!("创建重采样器失败: {}", e))?;

    let mut output: Vec<Vec<f32>> = vec![Vec::new(); num_channels];
    let mut pos = 0;

    while pos < total_frames {
        let end = (pos + chunk_size).min(total_frames);
        let actual_chunk = end - pos;

        // Prepare chunk, pad with zeros if partial
        let chunk: Vec<Vec<f32>> = pcm
            .samples
            .iter()
            .map(|ch| {
                let mut v = ch[pos..end].to_vec();
                if v.len() < chunk_size {
                    v.resize(chunk_size, 0.0);
                }
                v
            })
            .collect();

        let resampled = resampler
            .process(&chunk, None)
            .map_err(|e| format!("重采样失败: {}", e))?;

        // For the last partial chunk, truncate output proportionally
        let output_frames = if pos + chunk_size > total_frames {
            ((actual_chunk as f64 * 44100.0 / pcm.sample_rate as f64).round() as usize)
                .min(resampled[0].len())
        } else {
            resampled[0].len()
        };

        for (i, ch) in resampled.iter().enumerate() {
            output[i].extend_from_slice(&ch[..output_frames]);
        }

        pos += chunk_size;
    }

    let new_duration = if !output[0].is_empty() {
        output[0].len() as f64 / 44100.0
    } else {
        pcm.duration_secs
    };

    Ok(decoder::PcmData {
        samples: output,
        sample_rate: 44100,
        channels: num_channels,
        duration_secs: new_duration,
    })
}

/// Interleave planar stereo samples: [L0,L1,...] + [R0,R1,...] → [L0,R0,L1,R1,...]
fn interleave(pcm: &decoder::PcmData) -> Vec<f32> {
    if pcm.samples.len() < 2 || pcm.samples[0].is_empty() {
        return Vec::new();
    }
    let num_frames = pcm.samples[0].len();
    let mut interleaved = Vec::with_capacity(num_frames * 2);
    for i in 0..num_frames {
        interleaved.push(pcm.samples[0][i]);
        interleaved.push(pcm.samples[1][i]);
    }
    interleaved
}

/// Encode interleaved f32 stereo PCM to MP3 file using LAME encoder.
fn encode_mp3(
    pcm: &[f32],
    num_channels: usize,
    sample_rate: u32,
    output_path: &str,
) -> Result<(), String> {
    let mut encoder = mp3lame_encoder::Builder::new()
        .map_err(|e| format!("初始化编码器失败: {}", e))?
        .set_num_channels(num_channels)
        .map_err(|e| format!("设置声道数失败: {}", e))?
        .set_sample_rate(sample_rate)
        .map_err(|e| format!("设置采样率失败: {}", e))?
        .set_brate(mp3lame_encoder::Bitrate::Kbps192)
        .map_err(|e| format!("设置比特率失败: {}", e))?
        .set_quality(mp3lame_encoder::Quality::Best)
        .map_err(|e| format!("设置质量失败: {}", e))?
        .build()
        .map_err(|e| format!("构建编码器失败: {}", e))?;

    let input = mp3lame_encoder::InterleavedPcm(pcm);

    // Allocate output buffer: worst case ~1.25x input + 7200 bytes
    let buf_size = pcm.len() / num_channels * 5 / 4 + 7200;
    let mut mp3_buf = vec![0u8; buf_size];

    let encoded = encoder
        .encode(input, &mut mp3_buf)
        .map_err(|e| format!("MP3 编码失败: {}", e))?;

    // Flush remaining frames
    let mut flush_buf = vec![0u8; 7200];
    let flushed = encoder
        .flush::<mp3lame_encoder::FlushNoGap>(&mut flush_buf)
        .map_err(|e| format!("MP3 刷新失败: {}", e))?;

    // Write to file
    use std::io::Write;
    let mut file =
        std::fs::File::create(output_path).map_err(|e| format!("无法创建输出文件: {}", e))?;
    file.write_all(&mp3_buf[..encoded])
        .map_err(|e| format!("写入失败: {}", e))?;
    file.write_all(&flush_buf[..flushed])
        .map_err(|e| format!("写入失败: {}", e))?;

    Ok(())
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/converter.rs
git commit -m "feat: rewrite converter with symphonia/rubato/mp3lame pipeline"
```

---

### Task 4: Simplify audio_meta.rs

**Files:**
- Modify: `src-tauri/src/audio_meta.rs`

- [ ] **Step 1: Remove ffmpeg parsing functions, keep format_duration**

Replace the entire file:

```rust
/// Format duration in seconds to "M:SS" display string
pub fn format_duration(secs: f64) -> String {
    let total_secs = secs as u64;
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    format!("{}:{:02}", minutes, seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(272.15), "4:32");
        assert_eq!(format_duration(65.0), "1:05");
        assert_eq!(format_duration(5.0), "0:05");
        assert_eq!(format_duration(0.0), "0:00");
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/audio_meta.rs
git commit -m "refactor: remove ffmpeg stderr parsing from audio_meta"
```

---

### Task 5: Update commands.rs — Remove shell dependency

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Update commands to use new converter API (no AppHandle for scan)**

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::State;

use crate::converter;
use crate::models::*;

/// Shared state for tracking active conversion and cancellation
pub struct AppState {
    pub cancel_flag: Arc<AtomicBool>,
    pub converting: Arc<AtomicBool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            cancel_flag: Arc::new(AtomicBool::new(false)),
            converting: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[tauri::command]
pub fn scan_audio_files(input_dir: String) -> Result<Vec<AudioFile>, String> {
    converter::scan_directory(&input_dir)
}

#[tauri::command]
pub fn start_conversion(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    files: Vec<AudioFile>,
    output_dir: String,
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
        converter::convert_files(&app_handle, &files, &output_dir, &cancel_flag);
        converting.store(false, Ordering::Relaxed);
    });

    Ok(())
}

#[tauri::command]
pub fn open_folder(path: String) -> Result<(), String> {
    let path = std::path::Path::new(&path);
    if !path.exists() {
        std::fs::create_dir_all(path).map_err(|e| format!("无法创建文件夹: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("无法打开文件夹: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("无法打开文件夹: {}", e))?;
    }

    Ok(())
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "refactor: remove shell dependency from commands"
```

---

### Task 6: Update lib.rs — Register new module, remove shell plugin

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Update module declarations and plugin registration**

```rust
mod audio_meta;
mod commands;
mod converter;
mod decoder;
mod models;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::scan_audio_files,
            commands::start_conversion,
            commands::open_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "refactor: add decoder module, remove tauri_plugin_shell"
```

---

### Task 7: Update config files

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: Remove externalBin from tauri.conf.json**

Remove the line `"externalBin": ["binaries/ffmpeg"],` from the `bundle` section. The bundle section should become:

```json
"bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "windows": {
      "webviewInstallMode": {
        "type": "downloadBootstrapper"
      }
    }
  }
```

- [ ] **Step 2: Remove shell permissions from capabilities/default.json**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "dialog:allow-open"
  ]
}
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json src-tauri/capabilities/default.json
git commit -m "chore: remove ffmpeg sidecar config and shell permissions"
```

---

### Task 8: Update build/package scripts

**Files:**
- Modify: `scripts/build.sh`
- Rewrite: `scripts/package.sh`

- [ ] **Step 1: Update build.sh — remove ffmpeg validation**

Remove the ffmpeg sidecar check block (lines checking for `ffmpeg-$TARGET.exe`). The Windows cross-compilation section becomes:

```bash
        ENV_PREFIX="RUSTUP_TOOLCHAIN=nightly PATH=\"$LLVM_BIN:\$PATH\""

        if [[ "$BUNDLE" == false ]]; then
            BUILD_ARGS+=("--no-bundle")
            echo "  ⚠️  Skipping installer bundle (requires makensis on Windows)"
            echo "  📦 Building exe only"
        fi
```

Also remove the ffmpeg download reference from the usage text.

- [ ] **Step 2: Rewrite package.sh without sidecar**

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

usage() {
    echo "Usage: $0 [--target <triple>]"
    echo ""
    echo "Package the built application for distribution."
    echo ""
    echo "Options:"
    echo "  --target <triple>  Package for specific target (default: current platform)"
    exit 0
}

TARGET=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --target) TARGET="$2"; shift 2 ;;
        --help|-h) usage ;;
        *) echo "Unknown option: $1"; usage ;;
    esac
done

if [[ -z "$TARGET" ]]; then
    OS="$(uname -s)"
    ARCH="$(uname -m)"
    case "$OS" in
        Darwin)
            if [ "$ARCH" = "arm64" ]; then
                TARGET="aarch64-apple-darwin"
            else
                TARGET="x86_64-apple-darwin"
            fi
            ;;
        MINGW*|MSYS*|CYGWIN*)
            TARGET="x86_64-pc-windows-msvc"
            ;;
        *)
            echo "Unsupported platform: $OS"
            exit 1
            ;;
    esac
fi

RELEASE_DIR="$PROJECT_DIR/src-tauri/target/$TARGET/release"

case "$TARGET" in
    *-pc-windows-msvc)
        APP_NAME="frog-voice.exe"
        ARCHIVE_NAME="frog-voice-windows-x64"
        ;;
    *-apple-darwin)
        APP_NAME="frog-voice"
        ARCHIVE_NAME="frog-voice-macos-${TARGET%%-*}"
        ;;
    *)
        APP_NAME="frog-voice"
        ARCHIVE_NAME="frog-voice-$TARGET"
        ;;
esac

if [[ ! -f "$RELEASE_DIR/$APP_NAME" ]]; then
    echo "❌ Application not found: $RELEASE_DIR/$APP_NAME"
    echo "   Run: scripts/build.sh --target $TARGET"
    exit 1
fi

PACKAGE_DIR="$PROJECT_DIR/dist/$ARCHIVE_NAME"
rm -rf "$PACKAGE_DIR"
mkdir -p "$PACKAGE_DIR"

cp "$RELEASE_DIR/$APP_NAME" "$PACKAGE_DIR/"

echo "📦 Packaging $ARCHIVE_NAME..."
echo "   App: $APP_NAME"

cd "$PROJECT_DIR/dist"

case "$TARGET" in
    *-pc-windows-msvc)
        if command -v zip &>/dev/null; then
            zip -r "$ARCHIVE_NAME.zip" "$ARCHIVE_NAME/" > /dev/null
            echo ""
            echo "✅ Created: dist/$ARCHIVE_NAME.zip"
        else
            echo "⚠️  zip not found, skipping archive."
        fi
        ;;
    *-apple-darwin)
        zip -r "$ARCHIVE_NAME.zip" "$ARCHIVE_NAME/" > /dev/null
        echo ""
        echo "✅ Created: dist/$ARCHIVE_NAME.zip"
        ;;
    *)
        tar -czf "$ARCHIVE_NAME.tar.gz" "$ARCHIVE_NAME/"
        echo ""
        echo "✅ Created: dist/$ARCHIVE_NAME.tar.gz"
        ;;
esac

echo ""
echo "Contents:"
ls -lh "$PACKAGE_DIR/"
```

- [ ] **Step 3: Commit**

```bash
git add scripts/build.sh scripts/package.sh
git commit -m "chore: simplify build/package scripts without ffmpeg sidecar"
```

---

### Task 9: Remove sidecar files and update package.json

**Files:**
- Delete: `src-tauri/binaries/ffmpeg-*`
- Delete: `scripts/download-ffmpeg.sh`
- Modify: `package.json`

- [ ] **Step 1: Delete sidecar binaries and download script**

```bash
rm -f src-tauri/binaries/ffmpeg-aarch64-apple-darwin
rm -f src-tauri/binaries/ffmpeg-x86_64-apple-darwin
rm -f src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe
rm -f scripts/download-ffmpeg.sh
```

- [ ] **Step 2: Remove @tauri-apps/plugin-shell from package.json**

Remove the line `"@tauri-apps/plugin-shell": "^2",` from dependencies.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "chore: remove ffmpeg sidecar binaries and download script"
```

---

### Task 10: Build and test

- [ ] **Step 1: Run cargo build to verify compilation**

```bash
cd src-tauri && cargo build 2>&1
```

Expected: Successful compilation. If there are API mismatches (method names, type signatures), fix them based on compiler errors and re-run.

- [ ] **Step 2: Fix any compilation errors**

Common issues to watch for:
- `mp3lame_encoder::Builder` method names might differ (e.g., `set_sample_rate` vs `with_sample_rate`)
- `rubato::SincFixedIn::new` parameter order might differ
- `symphonia` import paths

- [ ] **Step 3: Run `pnpm tauri dev` to test the app**

Launch the app, select a folder with audio files, verify:
1. File scanning works (durations detected)
2. Conversion to MP3 works
3. Progress events are emitted
4. Output MP3 files are valid and playable

- [ ] **Step 4: Commit any fixes**

```bash
git add -A
git commit -m "fix: resolve compilation issues after ffmpeg migration"
```

---

## Self-Review Checklist

- [x] **Spec coverage:** Every section of the spec maps to a task
- [x] **Placeholder scan:** No "TBD", "TODO", or vague steps
- [x] **Type consistency:** PcmData, AudioFile, event types are consistent across tasks
- [x] **Format support:** SUPPORTED_EXTENSIONS matches symphonia capabilities (wav, mp3, flac, aac, ogg, oga, m4a, aiff, aif, mp4, m4v)
- [x] **Progress reporting:** decode_to_pcm callback → convert_single_file emits progress events → frontend unchanged
- [x] **Error handling:** Per-file errors don't stop batch conversion
- [x] **Frontend impact:** Zero changes needed
