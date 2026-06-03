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
