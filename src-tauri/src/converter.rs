use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::time::Instant;

use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use tauri::{AppHandle, Emitter};

use crate::decoder;
use crate::models::*;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "wav", "mp3", "flac", "aac", "ogg", "oga", "m4a", "aiff", "aif", "mp4", "m4v",
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
            ConvertDoneEvent {
                total,
                success: 0,
                failed: total,
            },
        );
        return ConvertDoneEvent {
            total,
            success: 0,
            failed: total,
        };
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

        match convert_single_file(app, &file.path, &output_path, cancel_flag) {
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

    let event = ConvertDoneEvent {
        total,
        success,
        failed,
    };
    let _ = app.emit("convert-done", &event);
    event
}

/// Convert a single audio file to MP3.
fn convert_single_file(
    app: &AppHandle,
    input_path: &str,
    output_path: &str,
    cancel_flag: &AtomicBool,
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
        0 => vec![Vec::new(), Vec::new()],
        1 => vec![pcm.samples[0].clone(), pcm.samples[0].clone()],
        2 => pcm.samples,
        _ => vec![pcm.samples[0].clone(), pcm.samples[1].clone()],
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

    let ratio = 44100.0 / pcm.sample_rate as f64;
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
/// Processes in chunks of 1152 samples (one MP3 frame) for correctness.
fn encode_mp3(
    pcm: &[f32],
    num_channels: usize,
    sample_rate: u32,
    output_path: &str,
) -> Result<(), String> {
    use std::io::Write;

    let mut encoder = mp3lame_encoder::Builder::new()
        .ok_or("初始化编码器失败")?
        .with_num_channels(num_channels as u8)
        .map_err(|e| format!("设置声道数失败: {}", e))?
        .with_sample_rate(sample_rate)
        .map_err(|e| format!("设置采样率失败: {}", e))?
        .with_brate(mp3lame_encoder::Bitrate::Kbps192)
        .map_err(|e| format!("设置比特率失败: {}", e))?
        .with_quality(mp3lame_encoder::Quality::Best)
        .map_err(|e| format!("设置质量失败: {}", e))?
        .build()
        .map_err(|e| format!("构建编码器失败: {}", e))?;

    let mut file =
        std::fs::File::create(output_path).map_err(|e| format!("无法创建输出文件: {}", e))?;

    // Encode in chunks of 1152 samples per channel (one MP3 frame at any sample rate)
    let frames_per_chunk = 1152;
    let total_frames = pcm.len() / num_channels;

    // Allocate buffer once — len=0, capacity=buf_size so spare_capacity_mut works correctly
    let buf_size = mp3lame_encoder::max_required_buffer_size(frames_per_chunk);
    let mut mp3_buf: Vec<u8> = Vec::with_capacity(buf_size);

    let mut pos = 0;
    while pos < total_frames {
        let end = (pos + frames_per_chunk).min(total_frames);
        let chunk = &pcm[pos * num_channels..end * num_channels];
        let input = mp3lame_encoder::InterleavedPcm(chunk);
        let n = encoder
            .encode(input, mp3_buf.spare_capacity_mut())
            .map_err(|e| format!("MP3 编码失败: {}", e))?;
        unsafe {
            mp3_buf.set_len(n);
        }
        file.write_all(&mp3_buf)
            .map_err(|e| format!("写入失败: {}", e))?;
        mp3_buf.clear(); // reset len to 0, keep capacity
        pos = end;
    }

    // Flush remaining frames
    let flushed = encoder
        .flush::<mp3lame_encoder::FlushNoGap>(mp3_buf.spare_capacity_mut())
        .map_err(|e| format!("MP3 刷新失败: {}", e))?;
    unsafe {
        mp3_buf.set_len(flushed);
    }
    file.write_all(&mp3_buf)
        .map_err(|e| format!("写入失败: {}", e))?;

    Ok(())
}
