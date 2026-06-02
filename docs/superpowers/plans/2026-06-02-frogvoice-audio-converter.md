# FrogVoice 音频转 MP3 工具 — 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建一个 Tauri v2 桌面应用，支持拖拽文件夹批量转换音频为 MP3，ffmpeg 通过 Sidecar 打包在应用内。

**Architecture:** 纯 Rust 后端处理——Rust 通过 Tauri Sidecar 调用 ffmpeg，通过 Tauri Events 实时推送进度给 Vue 3 前端。前端只负责 UI 展示和用户交互。

**Tech Stack:** Tauri v2, Vue 3 (Composition API), TypeScript, Rust, ffmpeg (sidecar), tauri-plugin-shell, tauri-plugin-dialog

---

## File Structure

### 新增文件

| 文件 | 职责 |
|------|------|
| `src-tauri/src/models.rs` | 数据结构定义（AudioFile, ConvertStatus 等） |
| `src-tauri/src/audio_meta.rs` | 解析 ffmpeg 输出获取时长、解析进度 |
| `src-tauri/src/converter.rs` | ffmpeg sidecar 调用、转换队列、进度推送 |
| `src-tauri/src/commands.rs` | Tauri Commands（scan, convert, open_folder） |
| `scripts/download-ffmpeg.sh` | 下载 ffmpeg 二进制到 sidecar 目录 |
| `src/types.ts` | TypeScript 类型定义 |
| `src/composables/useDragDrop.ts` | 拖拽文件夹 composable |
| `src/composables/useConverter.ts` | 转换逻辑 composable |
| `src/components/DropZone.vue` | 拖拽区域组件 |
| `src/components/FileItem.vue` | 单个文件行 |
| `src/components/FileList.vue` | 文件列表容器 |
| `src/components/ActionBar.vue` | 底部操作栏 |

### 修改文件

| 文件 | 改动 |
|------|------|
| `src-tauri/Cargo.toml` | 添加 tauri-plugin-shell, tauri-plugin-dialog, tokio |
| `src-tauri/tauri.conf.json` | 窗口尺寸、sidecar 配置 |
| `src-tauri/capabilities/default.json` | shell + dialog 权限 |
| `src-tauri/src/lib.rs` | 注册插件和 commands |
| `package.json` | 添加 @tauri-apps/plugin-shell, @tauri-apps/plugin-dialog |
| `src/App.vue` | 完全重写为主界面 |
| `.gitignore` | 添加 sidecar 二进制 |

---

## Task 1: Rust Dependencies & Frontend Dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `package.json`

- [ ] **Step 1: 添加 Rust 依赖到 Cargo.toml**

在 `[dependencies]` 中添加 `tauri-plugin-shell`, `tauri-plugin-dialog`, `tokio`:

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
tauri-plugin-shell = "2"
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["time"] }
```

- [ ] **Step 2: 添加前端依赖到 package.json**

在 `dependencies` 中添加:

```json
"@tauri-apps/plugin-shell": "^2",
"@tauri-apps/plugin-dialog": "^2"
```

- [ ] **Step 3: 安装前端依赖**

Run: `pnpm install`
Expected: 安装成功，无错误

- [ ] **Step 4: 验证 Rust 编译**

Run: `cd src-tauri && cargo check`
Expected: 编译成功（可能有一些 unused import 警告，忽略）

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml package.json pnpm-lock.yaml
git commit -m "chore: add shell, dialog plugins and tokio dependencies"
```

---

## Task 2: Tauri Configuration

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: 更新 tauri.conf.json**

替换整个文件为以下内容（更新窗口尺寸、标题、sidecar 配置）:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "frog-voice",
  "version": "0.1.0",
  "identifier": "cn.basecrypto.frog-voice",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "🐸 FrogVoice",
        "width": 720,
        "height": 560,
        "minWidth": 600,
        "minHeight": 480,
        "center": true
      }
    ],
    "security": {
      "csp": null
    }
  },
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
    "externalBin": ["binaries/ffmpeg"]
  }
}
```

- [ ] **Step 2: 更新 capabilities/default.json**

添加 shell 和 dialog 权限:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "shell:allow-spawn",
    "shell:allow-kill",
    "dialog:allow-open"
  ]
}
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json src-tauri/capabilities/default.json
git commit -m "chore: configure window size, sidecar, and permissions"
```

---

## Task 3: Rust Data Models

**Files:**
- Create: `src-tauri/src/models.rs`

- [ ] **Step 1: 创建 models.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub duration_secs: Option<f64>,
    pub extension: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FileConvertStatus {
    Pending,
    Converting { progress: f64 },
    Completed { output_size: u64 },
    Failed { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertProgressEvent {
    pub file_path: String,
    pub progress: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertStatusEvent {
    pub file_path: String,
    pub status: FileConvertStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertDoneEvent {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/models.rs
git commit -m "feat: add Rust data models for audio files and conversion status"
```

---

## Task 4: Audio Metadata & FFmpeg Progress Parsing (TDD)

**Files:**
- Create: `src-tauri/src/audio_meta.rs`

- [ ] **Step 1: 写失败的测试**

```rust
/// Parse duration from ffmpeg `-i` stderr output
pub fn parse_duration(ffmpeg_output: &str) -> Option<f64> {
    todo!()
}

/// Format duration in seconds to "M:SS" display string
pub fn format_duration(secs: f64) -> String {
    todo!()
}

/// Parse conversion progress from ffmpeg stderr line
/// Returns progress as 0.0-1.0 fraction
pub fn parse_progress(line: &str, total_duration_secs: f64) -> Option<f64> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_standard() {
        let output = "ffmpeg version 6.0 Copyright (c) 2000-2023\n  Duration: 00:04:32.15, start: 0.000000, bitrate: 128 kb/s\n";
        assert_eq!(parse_duration(output), Some(272.15));
    }

    #[test]
    fn test_parse_duration_short() {
        let output = "  Duration: 00:00:05.00, start: 0.000000\n";
        assert_eq!(parse_duration(output), Some(5.0));
    }

    #[test]
    fn test_parse_duration_not_found() {
        let output = "ffmpeg version 6.0\nNo such file or directory\n";
        assert_eq!(parse_duration(output), None);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(272.15), "4:32");
        assert_eq!(format_duration(65.0), "1:05");
        assert_eq!(format_duration(5.0), "0:05");
        assert_eq!(format_duration(0.0), "0:00");
    }

    #[test]
    fn test_parse_progress() {
        let line = "size=    1024kB time=00:02:15.50 bitrate=  61.8kbits/s speed=  128x";
        assert_eq!(parse_progress(line, 272.15), Some(135.5 / 272.15));
    }

    #[test]
    fn test_parse_progress_no_time() {
        let line = "size=       0kB time=-00:00:00.00 bitrate=   0.0kbits/s";
        assert_eq!(parse_progress(line, 272.15), None);
    }

    #[test]
    fn test_parse_progress_zero_duration() {
        let line = "size=    1024kB time=00:00:05.00 bitrate=  61.8kbits/s";
        assert_eq!(parse_progress(line, 0.0), None);
    }
}
```

- [ ] **Step 2: 运行测试验证失败**

Run: `cd src-tauri && cargo test --lib audio_meta`
Expected: 编译失败（`todo!()` panic）

- [ ] **Step 3: 实现三个函数**

```rust
/// Parse duration from ffmpeg `-i` stderr output
pub fn parse_duration(ffmpeg_output: &str) -> Option<f64> {
    for line in ffmpeg_output.lines() {
        if let Some(start) = line.find("Duration: ") {
            // "Duration: HH:MM:SS.ms"
            let duration_part = &line[start + 10..];
            // Take up to the first comma or end of relevant part
            let end = duration_part.find(',').unwrap_or(duration_part.len());
            let duration_str = duration_part[..end].trim();
            let parts: Vec<&str> = duration_str.split(':').collect();
            if parts.len() == 3 {
                let hours: f64 = parts[0].parse().ok()?;
                let minutes: f64 = parts[1].parse().ok()?;
                let seconds: f64 = parts[2].parse().ok()?;
                return Some(hours * 3600.0 + minutes * 60.0 + seconds);
            }
        }
    }
    None
}

/// Format duration in seconds to "M:SS" display string
pub fn format_duration(secs: f64) -> String {
    let total_secs = secs as u64;
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    format!("{}:{:02}", minutes, seconds)
}

/// Parse conversion progress from ffmpeg stderr line
/// Returns progress as 0.0-1.0 fraction
pub fn parse_progress(line: &str, total_duration_secs: f64) -> Option<f64> {
    if total_duration_secs <= 0.0 {
        return None;
    }
    if let Some(start) = line.find("time=") {
        let time_part = &line[start + 5..];
        // "HH:MM:SS.ms"
        let end = time_part.find(' ').unwrap_or(time_part.len());
        let time_str = &time_part[..end];
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() == 3 {
            let hours: f64 = parts[0].parse().ok()?;
            let minutes: f64 = parts[1].parse().ok()?;
            let seconds: f64 = parts[2].parse().ok()?;
            let current = hours * 3600.0 + minutes * 60.0 + seconds;
            return Some((current / total_duration_secs).clamp(0.0, 1.0));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    // ... tests from step 1 (unchanged)
}
```

- [ ] **Step 4: 运行测试验证通过**

Run: `cd src-tauri && cargo test --lib audio_meta`
Expected: 所有 7 个测试通过

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/audio_meta.rs
git commit -m "feat: add audio metadata parsing with tests"
```

---

## Task 5: FFmpeg Converter Core

**Files:**
- Create: `src-tauri/src/converter.rs`

- [ ] **Step 1: 创建 converter.rs**

```rust
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
    let output = app
        .shell()
        .sidecar("ffmpeg")
        .ok()?
        .args(["-i", file_path])
        .output()
        .ok()?;

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
    if let Err(e) = std::fs::create_dir_all(output_dir) {
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

        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(CommandEvent::Stderr(line)) => {
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
            Ok(CommandEvent::Terminated(status)) => {
                exit_code = status.code();
                break;
            }
            Ok(CommandEvent::Stdout(_)) => {}
            Ok(_) => {}
            Err(_) => {
                // Channel closed without termination event
                break;
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
```

- [ ] **Step 2: 验证编译**

Run: `cd src-tauri && cargo check`
Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/converter.rs
git commit -m "feat: add ffmpeg converter core with progress tracking"
```

---

## Task 6: Tauri Commands

**Files:**
- Create: `src-tauri/src/commands.rs`

- [ ] **Step 1: 创建 commands.rs**

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex;

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
pub fn scan_audio_files(app: AppHandle, input_dir: String) -> Result<Vec<AudioFile>, String> {
    converter::scan_directory(&app, &input_dir)
}

#[tauri::command]
pub fn start_conversion(
    app: AppHandle,
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

- [ ] **Step 2: 验证编译**

Run: `cd src-tauri && cargo check`
Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add Tauri commands for scanning, conversion, and folder opening"
```

---

## Task 7: Wire lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 重写 lib.rs**

```rust
mod audio_meta;
mod commands;
mod converter;
mod models;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
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

- [ ] **Step 2: 验证编译**

Run: `cd src-tauri && cargo check`
Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: wire up plugins, commands, and state in lib.rs"
```

---

## Task 8: FFmpeg Download Script

**Files:**
- Create: `scripts/download-ffmpeg.sh`
- Modify: `.gitignore`

- [ ] **Step 1: 创建下载脚本**

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BINARIES_DIR="$PROJECT_DIR/src-tauri/binaries"

mkdir -p "$BINARIES_DIR"

detect_platform() {
    local os="$(uname -s)"
    local arch="$(uname -m)"
    case "$os" in
        Darwin)
            if [ "$arch" = "arm64" ]; then
                echo "aarch64-apple-darwin"
            else
                echo "x86_64-apple-darwin"
            fi
            ;;
        Linux)
            echo "x86_64-unknown-linux-gnu"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            echo "x86_64-pc-windows-msvc"
            ;;
        *)
            echo "unsupported" ;;
    esac
}

TARGET=$(detect_platform)
echo "Detected target: $TARGET"

case "$TARGET" in
    *-apple-darwin)
        echo "Downloading ffmpeg for macOS..."
        TMPFILE=$(mktemp /tmp/ffmpeg-XXXXXX.zip)
        curl -L -o "$TMPFILE" "https://evermeet.cx/ffmpeg/getrelease/ffmpeg/zip"
        unzip -o "$TMPFILE" -d "$BINARIES_DIR"
        mv "$BINARIES_DIR/ffmpeg" "$BINARIES_DIR/ffmpeg-$TARGET"
        chmod +x "$BINARIES_DIR/ffmpeg-$TARGET"
        rm -f "$TMPFILE"
        echo "Saved to $BINARIES_DIR/ffmpeg-$TARGET"
        ;;
    *-pc-windows-msvc)
        echo "Downloading ffmpeg for Windows..."
        TMPFILE=$(mktemp /tmp/ffmpeg-XXXXXX.zip)
        curl -L -o "$TMPFILE" "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"
        unzip -o "$TMPFILE" -d "$BINARIES_DIR"
        # Find the ffmpeg.exe in the extracted directory
        FFMPEG_EXE=$(find "$BINARIES_DIR" -name "ffmpeg.exe" | head -1)
        cp "$FFMPEG_EXE" "$BINARIES_DIR/ffmpeg-$TARGET.exe"
        rm -rf "$BINARIES_DIR/ffmpeg-"*"-essentials"
        rm -f "$TMPFILE"
        echo "Saved to $BINARIES_DIR/ffmpeg-$TARGET.exe"
        ;;
    *)
        echo "Unsupported platform. Please download ffmpeg manually."
        exit 1
        ;;
esac

echo "Done!"
```

- [ ] **Step 2: 添加 sidecar 二进制到 .gitignore**

在 `.gitignore` 末尾追加:

```
# Sidecar binaries
src-tauri/binaries/ffmpeg-*
```

- [ ] **Step 3: 运行下载脚本（当前平台）**

Run: `chmod +x scripts/download-ffmpeg.sh && ./scripts/download-ffmpeg.sh`
Expected: 下载完成，`src-tauri/binaries/ffmpeg-{target}` 文件存在

- [ ] **Step 4: 验证 ffmpeg 可执行**

Run: `./src-tauri/binaries/ffmpeg-* -version | head -1`
Expected: 显示 ffmpeg 版本号

- [ ] **Step 5: Commit**

```bash
git add scripts/download-ffmpeg.sh .gitignore
git commit -m "feat: add ffmpeg download script and gitignore for sidecar binaries"
```

---

## Task 9: Frontend TypeScript Types

**Files:**
- Create: `src/types.ts`

- [ ] **Step 1: 创建 types.ts**

```typescript
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
```

- [ ] **Step 2: Commit**

```bash
git add src/types.ts
git commit -m "feat: add TypeScript types and utility functions"
```

---

## Task 10: useDragDrop Composable

**Files:**
- Create: `src/composables/useDragDrop.ts`

- [ ] **Step 1: 创建 composables 目录和 useDragDrop.ts**

```typescript
import { ref } from 'vue';

export function useDragDrop() {
  const isDragging = ref(false);
  const path = ref('');
  const error = ref('');

  function onDragOver(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    isDragging.value = true;
  }

  function onDragLeave(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    isDragging.value = false;
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    isDragging.value = false;
    error.value = '';

    if (!e.dataTransfer?.files.length) return;

    // Tauri's WebView adds a `path` property to File objects
    const file = e.dataTransfer.files[0] as File & { path?: string };
    const filePath = file.path || file.name;

    if (!filePath) {
      error.value = '无法获取文件夹路径';
      return;
    }

    path.value = filePath;
  }

  return {
    isDragging,
    path,
    error,
    onDragOver,
    onDragLeave,
    onDrop,
  };
}
```

- [ ] **Step 2: Commit**

```bash
git add src/composables/useDragDrop.ts
git commit -m "feat: add drag-and-drop composable for folder selection"
```

---

## Task 11: useConverter Composable

**Files:**
- Create: `src/composables/useConverter.ts`

- [ ] **Step 1: 创建 useConverter.ts**

```typescript
import { ref, reactive } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  AudioFile,
  FileEntry,
  ConvertProgressEvent,
  ConvertStatusEvent,
  ConvertDoneEvent,
  FileConvertStatus,
} from '../types';

export function useConverter() {
  const files = reactive<FileEntry[]>([]);
  const isConverting = ref(false);
  const isScanning = ref(false);
  const doneSummary = ref<ConvertDoneEvent | null>(null);

  let unlisteners: UnlistenFn[] = [];

  async function startListening() {
    // Clean up previous listeners
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

  const error = ref('');

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
```

- [ ] **Step 2: Commit**

```bash
git add src/composables/useConverter.ts
git commit -m "feat: add converter composable with event listening"
```

---

## Task 12: DropZone Component

**Files:**
- Create: `src/components/DropZone.vue`

- [ ] **Step 1: 创建 DropZone.vue**

```vue
<script setup lang="ts">
import { computed } from 'vue';
import { useDragDrop } from '../composables/useDragDrop';
import { open } from '@tauri-apps/plugin-dialog';

const props = defineProps<{
  label: string;
  icon: string;
  modelValue: string;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

const { isDragging, onDragOver, onDragLeave, onDrop } = useDragDrop();

const displayPath = computed(() => props.modelValue || '拖拽文件夹到此处');

function handleDrop(e: DragEvent) {
  onDrop(e);
  if (isDragging.value === false) {
    // useDragDrop sets path on successful drop
    // We need to read the path from the drop
    const file = e.dataTransfer?.files[0] as File & { path?: string };
    if (file?.path) {
      emit('update:modelValue', file.path);
    }
  }
}

async function browse() {
  const selected = await open({ directory: true, multiple: false });
  if (selected) {
    emit('update:modelValue', selected);
  }
}
</script>

<template>
  <div
    class="drop-zone"
    :class="{ active: isDragging, 'has-path': modelValue }"
    @dragover="onDragOver"
    @dragleave="onDragLeave"
    @drop="handleDrop"
    @click="browse"
  >
    <div class="drop-icon">{{ icon }}</div>
    <div class="drop-label">{{ label }}</div>
    <div class="drop-hint">{{ displayPath }}</div>
  </div>
</template>

<style scoped>
.drop-zone {
  border: 2px dashed var(--border-color);
  border-radius: 10px;
  padding: 24px 16px;
  text-align: center;
  background: var(--surface-color);
  cursor: pointer;
  transition: all 0.2s ease;
}

.drop-zone:hover {
  border-color: var(--primary-color);
  background: var(--surface-hover);
}

.drop-zone.active {
  border-color: var(--primary-color);
  background: var(--primary-dim);
  transform: scale(1.02);
}

.drop-zone.has-path {
  border-color: var(--success-color);
  border-style: solid;
}

.drop-icon {
  font-size: 32px;
  margin-bottom: 8px;
}

.drop-label {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 4px;
  color: var(--text-primary);
}

.drop-hint {
  font-size: 11px;
  color: var(--text-secondary);
  word-break: break-all;
  max-height: 32px;
  overflow: hidden;
}
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/DropZone.vue
git commit -m "feat: add DropZone component with drag-and-drop and browse"
```

---

## Task 13: FileItem Component

**Files:**
- Create: `src/components/FileItem.vue`

- [ ] **Step 1: 创建 FileItem.vue**

```vue
<script setup lang="ts">
import type { FileEntry } from '../types';
import { formatDuration, formatFileSize, getStatusText, getStatusClass } from '../types';

defineProps<{ entry: FileEntry }>();
</script>

<template>
  <div class="file-item" :class="getStatusClass(entry.status)">
    <div class="file-info">
      <div class="file-name">🎵 {{ entry.file.name }}</div>
      <div class="file-meta">
        {{ formatDuration(entry.file.duration_secs) }} · {{ formatFileSize(entry.file.size) }}
      </div>
    </div>
    <div class="file-format">192kbps → MP3</div>
    <div class="file-status">
      <template v-if="entry.status.type === 'Converting'">
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: (entry.status.progress * 100) + '%' }"></div>
        </div>
        <span class="status-text converting">{{ getStatusText(entry.status) }}</span>
      </template>
      <span v-else class="status-text" :class="getStatusClass(entry.status)">
        {{ getStatusText(entry.status) }}
      </span>
    </div>
    <div class="file-output">
      <template v-if="entry.status.type === 'Completed'">
        {{ formatFileSize(entry.status.output_size) }}
      </template>
      <template v-else>—</template>
    </div>
  </div>
</template>

<style scoped>
.file-item {
  display: grid;
  grid-template-columns: 1fr 100px 120px 70px;
  align-items: center;
  padding: 10px 16px;
  border-bottom: 1px solid var(--border-color);
  border-left: 3px solid var(--border-color);
  transition: background 0.2s;
}

.file-item:last-child {
  border-bottom: none;
}

/* Status border colors */
.file-item.status-pending {
  border-left-color: var(--text-secondary);
}
.file-item.status-converting {
  border-left-color: var(--primary-color);
  background: var(--primary-dim);
}
.file-item.status-completed {
  border-left-color: var(--success-color);
  background: var(--success-dim);
}
.file-item.status-failed {
  border-left-color: var(--error-color);
  background: var(--error-dim);
}

.file-info {
  min-width: 0;
}

.file-name {
  font-size: 13px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.file-meta {
  font-size: 11px;
  color: var(--text-secondary);
  margin-top: 2px;
}

.file-format {
  font-size: 11px;
  color: var(--text-secondary);
}

.file-status {
  font-size: 11px;
}

.file-output {
  font-size: 11px;
  color: var(--text-secondary);
  text-align: right;
}

.progress-bar {
  background: var(--border-color);
  border-radius: 3px;
  height: 4px;
  width: 80px;
  margin-bottom: 4px;
}

.progress-fill {
  background: var(--primary-color);
  height: 100%;
  border-radius: 3px;
  transition: width 0.2s;
}

.status-text {
  font-size: 11px;
}

.status-text.status-completed {
  color: var(--success-color);
}

.status-text.status-failed {
  color: var(--error-color);
}

.status-text.converting {
  color: var(--primary-color);
}
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/FileItem.vue
git commit -m "feat: add FileItem component with status display and progress bar"
```

---

## Task 14: FileList Component

**Files:**
- Create: `src/components/FileList.vue`

- [ ] **Step 1: 创建 FileList.vue**

```vue
<script setup lang="ts">
import type { FileEntry } from '../types';
import FileItem from './FileItem.vue';

defineProps<{ files: FileEntry[] }>();
</script>

<template>
  <div class="file-list" v-if="files.length > 0">
    <div class="file-list-header">
      <span class="header-title">待转换文件 <span class="header-count">({{ files.length }} 个文件)</span></span>
    </div>
    <div class="file-list-body">
      <FileItem v-for="entry in files" :key="entry.file.path" :entry="entry" />
    </div>
  </div>
  <div v-else class="file-list-empty">
    <p>请选择输入文件夹以扫描音频文件</p>
  </div>
</template>

<style scoped>
.file-list {
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid var(--border-color);
}

.file-list-header {
  padding: 10px 16px;
  background: var(--surface-color);
  border-bottom: 1px solid var(--border-color);
}

.header-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.header-count {
  font-weight: 400;
  color: var(--text-secondary);
}

.file-list-body {
  max-height: 300px;
  overflow-y: auto;
}

.file-list-empty {
  text-align: center;
  padding: 32px;
  color: var(--text-secondary);
  border: 1px dashed var(--border-color);
  border-radius: 8px;
  font-size: 14px;
}
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/FileList.vue
git commit -m "feat: add FileList component"
```

---

## Task 15: ActionBar Component

**Files:**
- Create: `src/components/ActionBar.vue`

- [ ] **Step 1: 创建 ActionBar.vue**

```vue
<script setup lang="ts">
import type { ConvertDoneEvent, FileEntry } from '../types';

defineProps<{
  files: FileEntry[];
  isConverting: boolean;
  outputDir: string;
  doneSummary: ConvertDoneEvent | null;
}>();

const emit = defineEmits<{
  start: [];
  openFolder: [];
}>();
</script>

<template>
  <div class="action-bar">
    <div class="action-summary">
      <template v-if="doneSummary">
        进度：{{ doneSummary.success }}/{{ doneSummary.total }} 完成
        <span v-if="doneSummary.failed > 0" class="failed-count">，{{ doneSummary.failed }} 失败</span>
      </template>
      <template v-else-if="isConverting">
        正在转换中...
      </template>
      <template v-else-if="files.length > 0">
        已就绪，{{ files.length }} 个文件等待转换
      </template>
    </div>
    <div class="action-buttons">
      <button
        v-if="outputDir && doneSummary"
        class="btn btn-secondary"
        @click="emit('openFolder')"
      >
        打开输出文件夹
      </button>
      <button
        v-if="files.length > 0 && !isConverting && !doneSummary"
        class="btn btn-primary"
        @click="emit('start')"
      >
        开始转换
      </button>
    </div>
  </div>
</template>

<style scoped>
.action-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 12px;
}

.action-summary {
  font-size: 12px;
  color: var(--text-secondary);
}

.failed-count {
  color: var(--error-color);
}

.action-buttons {
  display: flex;
  gap: 8px;
}

.btn {
  padding: 6px 14px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  border: none;
  transition: all 0.2s;
}

.btn-primary {
  background: var(--success-color);
  color: #000;
}

.btn-primary:hover {
  filter: brightness(1.1);
}

.btn-secondary {
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border-color);
}

.btn-secondary:hover {
  border-color: var(--text-secondary);
}
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/ActionBar.vue
git commit -m "feat: add ActionBar component"
```

---

## Task 16: App.vue Integration & Dark Theme

**Files:**
- Rewrite: `src/App.vue`

- [ ] **Step 1: 重写 App.vue**

```vue
<script setup lang="ts">
import { ref, watch } from 'vue';
import DropZone from './components/DropZone.vue';
import FileList from './components/FileList.vue';
import ActionBar from './components/ActionBar.vue';
import { useConverter } from './composables/useConverter';

const inputDir = ref('');
const outputDir = ref('');

const {
  files,
  isConverting,
  doneSummary,
  error,
  scanFiles,
  startConversion,
  openFolder,
  reset,
} = useConverter();

// When inputDir changes, scan for files
watch(inputDir, async (newDir) => {
  if (newDir) {
    await scanFiles(newDir);
  } else {
    reset();
  }
});

// Reset when outputDir is cleared
watch(outputDir, (newDir) => {
  if (!newDir) {
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
      />
      <DropZone
        v-model="outputDir"
        label="输出文件夹"
        icon="📂"
      />
    </div>

    <div v-if="error" class="error-banner">{{ error }}</div>

    <div class="content">
      <FileList :files="files" />
      <ActionBar
        :files="files"
        :is-converting="isConverting"
        :output-dir="outputDir"
        :done-summary="doneSummary"
        @start="handleStart"
        @open-folder="handleOpenFolder"
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
</style>
```

- [ ] **Step 2: 验证前端编译**

Run: `pnpm build`
Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
git add src/App.vue
git commit -m "feat: rewrite App.vue with dark theme and full integration"
```

---

## Task 17: End-to-End Test

- [ ] **Step 1: 启动开发服务器**

Run: `pnpm tauri dev`
Expected: 应用窗口打开，显示深色主题界面

- [ ] **Step 2: 测试拖拽功能**

准备一个包含几个音频文件的测试文件夹（wav, flac 等），拖拽到输入文件夹区域。
Expected: 拖拽区变绿，显示文件路径，下方出现文件列表

- [ ] **Step 3: 测试扫描功能**

验证文件列表显示正确的文件名、时长、大小。
Expected: 列表正确显示，所有文件状态为"等待中"

- [ ] **Step 4: 测试转换功能**

拖拽一个输出文件夹，点击"开始转换"。
Expected: 文件逐个显示蓝色进度条，完成后变绿，失败文件变红

- [ ] **Step 5: 测试打开文件夹**

转换完成后点击"打开输出文件夹"。
Expected: 系统文件管理器打开输出目录，MP3 文件存在且可播放

- [ ] **Step 6: Commit（如有修复）**

```bash
git add -A
git commit -m "fix: address issues found during e2e testing"
```

---

## Self-Review Checklist

- [x] **Spec coverage**: 所有 spec 需求都有对应 Task
  - ✅ 音频格式扫描（含 mp3）→ Task 5, 9
  - ✅ ffmpeg 参数 → Task 5
  - ✅ 拖拽文件夹 → Task 10, 12
  - ✅ 文件列表（元信息+状态+进度条+打开文件夹）→ Task 13, 14, 15
  - ✅ 串行转换 → Task 5
  - ✅ 超时保护 → Task 5
  - ✅ 防重复触发 → Task 6
  - ✅ Sidecar 打包 → Task 2, 8
  - ✅ macOS + Windows → Task 6, 8
  - ✅ 错误处理 → Task 5, 6
  - ✅ 窗口配置 → Task 2
  - ✅ 进度节流 200ms → Task 5
  - ✅ 深色主题 → Task 16
  - ✅ 中文界面 → Task 16
- [x] **Placeholder scan**: 无 TBD/TODO
- [x] **Type consistency**: models.rs 的 `AudioFile.duration_secs` 与 types.ts 的 `duration_secs` 一致；`FileConvertStatus` 的 tag=type 序列化与 TypeScript 的 discriminated union 匹配
