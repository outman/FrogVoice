use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

use crate::audio_meta;
use crate::models::*;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "wav", "mp3", "flac", "aac", "ogg", "oga", "wma", "m4a",
    "aiff", "aif", "ape", "opus", "dsf", "ac3", "amr", "au",
    "snd", "mid", "midi", "wv", "tta",
];

const CONVERT_TIMEOUT_SECS: u64 = 600;
const PROGRESS_REPORT_INTERVAL: Duration = Duration::from_millis(200);

/// Check if a file has a supported audio extension
pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Get the duration of an audio file using ffmpeg `-i`
pub fn get_audio_duration(app: &AppHandle, file_path: &str) -> Option<f64> {
    let output = tauri::async_runtime::block_on(async {
        app.shell()
            .sidecar("ffmpeg")
            .ok()?
            .args(["-i", file_path])
            .output()
            .await
            .ok()
    })?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    audio_meta::parse_duration(&stderr)
}

/// Scan a directory for audio files
pub fn scan_directory(app: &AppHandle, input_dir: &str) -> Result<Vec<AudioFile>, String> {
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
            let metadata = std::fs::metadata(&path).map_err(|e| format!("无法读取文件信息: {}", e))?;
            let path_str = path.to_string_lossy().to_string();
            let duration = get_audio_duration(app, &path_str);

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

    // Ensure output directory exists
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

        // Emit started status
        let _ = app.emit(
            "convert-status",
            ConvertStatusEvent {
                file_path: file.path.clone(),
                status: FileConvertStatus::Converting { progress: 0.0 },
            },
        );

        match run_ffmpeg(app, &file.path, &output_path, file.duration_secs, cancel_flag) {
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

/// Run ffmpeg sidecar to convert a single file
fn run_ffmpeg(
    app: &AppHandle,
    input_path: &str,
    output_path: &str,
    duration: Option<f64>,
    cancel_flag: &AtomicBool,
) -> Result<u64, String> {
    let sidecar = app
        .shell()
        .sidecar("ffmpeg")
        .map_err(|e| format!("无法启动 ffmpeg: {}", e))?;

    let (mut rx, _child) = sidecar
        .args([
            "-y",
            "-i",
            input_path,
            "-vn",
            "-acodec",
            "libmp3lame",
            "-ar",
            "44100",
            "-ac",
            "2",
            "-b:a",
            "192k",
            output_path,
        ])
        .spawn()
        .map_err(|e| format!("无法启动 ffmpeg 进程: {}", e))?;

    let total_duration = duration.unwrap_or(0.0);
    let start = Instant::now();
    let mut last_progress_report = Instant::now();
    let mut exit_code: Option<i32> = None;

    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err("转换已取消".to_string());
        }

        // Check timeout
        if start.elapsed() > Duration::from_secs(CONVERT_TIMEOUT_SECS) {
            return Err("转换超时（10分钟）".to_string());
        }

        let event = tauri::async_runtime::block_on(async {
            tokio::time::timeout(Duration::from_millis(100), rx.recv()).await
        });

        match event {
            Ok(Some(CommandEvent::Stderr(line))) => {
                let line_str = String::from_utf8_lossy(&line);

                // Report progress at throttled intervals
                if total_duration > 0.0
                    && last_progress_report.elapsed() >= PROGRESS_REPORT_INTERVAL
                {
                    if let Some(progress) = audio_meta::parse_progress(&line_str, total_duration) {
                        let _ = app.emit(
                            "convert-progress",
                            ConvertProgressEvent {
                                file_path: input_path.to_string(),
                                progress,
                            },
                        );
                        last_progress_report = Instant::now();
                    }
                }
            }
            Ok(Some(CommandEvent::Terminated(status))) => {
                exit_code = status.code;
                break;
            }
            Ok(Some(CommandEvent::Stdout(_))) => {}
            Ok(Some(_)) => {}
            Ok(None) => {
                // Channel closed without termination event
                break;
            }
            Err(_) => {
                // Timeout elapsed, continue loop to check cancel/timeout
            }
        }
    }

    match exit_code {
        Some(0) => {
            let output_size = std::fs::metadata(output_path)
                .map(|m| m.len())
                .unwrap_or(0);
            Ok(output_size)
        }
        Some(code) => Err(format!("ffmpeg 退出码: {}", code)),
        None => Err("ffmpeg 进程异常终止".to_string()),
    }
}
