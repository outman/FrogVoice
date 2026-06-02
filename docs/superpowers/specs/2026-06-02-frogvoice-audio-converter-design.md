# FrogVoice 音频转 MP3 工具 — 设计文档

**日期**: 2026-06-02
**状态**: 待实现

## 概述

FrogVoice 是一个基于 Tauri v2 + Vue 3 的桌面音频批量转换工具，将各种音频格式统一转换为 MP3（192kbps, 44100Hz, 立体声）。ffmpeg 二进制文件通过 Tauri Sidecar 机制打包在应用内，用户无需额外安装。

## 需求

- **输入格式**: 所有常见音频格式（wav, mp3, flac, aac, ogg, wma, m4a, aiff, ape, opus, alac, dsf 等），包含 mp3 本身（应对改扩展名的情况）
- **输出格式**: MP3，参数 `-vn -acodec libmp3lame -ar 44100 -ac 2 -b:a 192k`
- **平台**: macOS（Intel + Apple Silicon）+ Windows 64-bit
- **界面语言**: 中文
- **主题**: 深色主题

## 架构

### 方案：纯 Rust 后端处理

转换逻辑全部在 Rust 侧完成。Rust 通过 Tauri Command 接收前端参数，使用 Tauri Sidecar 调用 ffmpeg，通过 Tauri Events 实时推送进度和状态给前端。

```
┌─────────────────────────────────────────────┐
│                  Vue 3 前端                   │
│  App.vue (拖拽区 + 文件列表 + 状态展示)        │
└──────────┬──────────────────┬────────────────┘
           │ Tauri Commands    │ Tauri Events
           ▼                  │
┌──────────────────────────────┴───────────────┐
│              Rust 后端 (lib.rs)               │
│                                              │
│  scan_audio_files()  → 扫描输入目录           │
│  start_conversion()  → 启动批量转换           │
│  get_file_metadata() → 读取音频元信息          │
│                                              │
│  通过 Tauri Sidecar 调用 ffmpeg              │
│  通过 Events 推送进度/状态给前端               │
└──────────────────────────────────────────────┘
```

### 数据流

1. 用户拖拽文件夹 → 前端拿到路径 → `invoke("scan_audio_files", { inputDir })` → Rust 扫描目录下所有音频文件 → 返回文件列表（含元信息：时长、大小）
2. 用户点击"开始转换" → `invoke("start_conversion", { files, outputDir })` → Rust 逐个调用 ffmpeg sidecar → 通过 Events 推送每个文件的进度/完成/失败状态
3. 前端监听 Events，实时更新列表状态和进度条

### ffmpeg Sidecar 集成

- 使用 `tauri-plugin-shell` 的 sidecar 功能
- 构建时自动下载对应平台的 ffmpeg 二进制文件
- macOS: `ffmpeg-x86_64-apple-darwin` + `ffmpeg-aarch64-apple-darwin`
- Windows: `ffmpeg-x86_64-pc-windows-msvc.exe`

## Rust 后端设计

### 模块结构

```
src-tauri/src/
  main.rs          — 入口（不变）
  lib.rs           — Tauri Builder，注册所有 commands 和插件
  commands.rs      — Tauri Commands 定义
  converter.rs     — ffmpeg 转换核心逻辑
  audio_meta.rs    — 音频文件元信息读取（时长、大小）
  models.rs        — 数据结构定义
```

### Tauri Commands

| Command | 入参 | 返回 | 说明 |
|---------|------|------|------|
| `scan_audio_files` | `input_dir: String` | `Vec<AudioFile>` | 扫描目录，返回音频文件列表 |
| `start_conversion` | `files: Vec<AudioFile>, output_dir: String` | — | 启动批量转换 |
| `open_folder` | `path: String` | — | 在系统文件管理器中打开文件夹 |

### Tauri Events（Rust → 前端推送）

| Event | Payload | 说明 |
|-------|---------|------|
| `convert-progress` | `{ file, progress: f64 }` | 单个文件转换进度 0-1 |
| `convert-status` | `{ file, status, error? }` | 文件状态变更：started/completed/failed |
| `convert-done` | `{ total, success, failed }` | 全部转换完成汇总 |

### 音频元信息获取

通过 ffmpeg sidecar 执行 `ffmpeg -i <file>` 解析 stderr 输出中的 Duration 信息，避免引入额外依赖。

### 支持的输入扩展名

`wav, mp3, flac, aac, ogg, oga, wma, m4a, aiff, aif, ape, opus, alac, dsf, ac3, amr, au, snd, mid, midi, wv, tta`

## 前端设计

### 组件结构

```
src/
  App.vue              — 根布局（深色背景）
  components/
    DropZone.vue       — 拖拽区域组件（输入/输出文件夹）
    FileList.vue       — 文件列表容器
    FileItem.vue       — 单个文件行（文件名、元信息、状态、进度条）
    ActionBar.vue      — 底部操作栏（开始转换、打开文件夹）
  composables/
    useConverter.ts    — 转换逻辑 composable（调用 Rust commands，监听 Events）
    useDragDrop.ts     — 拖拽逻辑 composable
  types.ts             — TypeScript 类型定义
```

### 组件交互流

```
App.vue
  ├── DropZone (输入) ──drag──→ 拿到路径 → invoke("scan_audio_files")
  ├── DropZone (输出) ──drag──→ 拿到路径
  ├── FileList
  │     └── FileItem × N  ←── 监听 convert-progress / convert-status events
  └── ActionBar
        ├── [开始转换] → invoke("start_conversion")
        └── [打开文件夹] → invoke("open_folder")
```

### UI 布局

- 顶部：两个并排拖拽区域（输入文件夹 / 输出文件夹）
- 中部：文件列表表格，每行包含：
  - 文件名 + 时长/大小
  - 目标格式
  - 状态（等待/转换中带进度条/成功/失败）
  - 输出文件大小
- 底部：进度汇总 + 操作按钮

### 状态色彩

- 🟢 绿色边框 + 绿色背景：已完成
- 🔵 蓝色边框 + 蓝色进度条：转换中
- 🔴 红色边框 + 红色背景：失败
- ⚪ 灰色边框：等待中

### 关键交互细节

- **拖拽文件夹**：监听 `dragover`/`drop` 事件，从 `dataTransfer` 中提取文件路径
- **进度计算**：Rust 端解析 ffmpeg stderr 中的 `time=` 字段与总时长对比计算进度百分比
- **状态色彩**：通过 CSS class 切换 `--status-color` 变量

## 错误处理

| 场景 | 处理方式 |
|------|---------|
| 输入路径不存在/不是文件夹 | 拖拽区显示红色错误提示 |
| 输出文件夹不存在 | 自动创建 |
| ffmpeg sidecar 未找到 | 启动时检测，弹窗提示用户重新安装 |
| 单个文件转换失败 | 标红该行，显示错误原因，继续转换下一个 |
| 输出文件已存在 | 直接覆盖（同名 mp3） |
| 文件无法读取（权限等） | 标红，提示权限错误 |
| 转换过程中用户关闭窗口 | 确认对话框，提示"正在转换中，确定退出？" |

## 转换队列逻辑

- **串行转换**：逐个文件调用 ffmpeg，避免多进程竞争资源
- **超时保护**：单个文件转换超时 10 分钟，超时则 kill 进程标记失败
- **防重复触发**：转换过程中禁用"开始转换"按钮
- **进度推送节流**：Rust 端每 200ms 推送一次 `convert-progress` event，避免前端渲染压力

## 窗口配置

- 默认尺寸：720 × 560
- 最小尺寸：600 × 480
- 标题栏：🐸 FrogVoice
