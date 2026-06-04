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
