#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BINARIES_DIR="$PROJECT_DIR/src-tauri/binaries"

mkdir -p "$BINARIES_DIR"

usage() {
    echo "Usage: $0 [--all | --target <triple>]"
    echo ""
    echo "Options:"
    echo "  --all              Download ffmpeg for all supported platforms"
    echo "  --target <triple>  Download for a specific target triple"
    echo "                     (e.g. aarch64-apple-darwin, x86_64-pc-windows-msvc)"
    echo ""
    echo "  (no args)          Auto-detect current platform"
    echo ""
    echo "Supported targets:"
    echo "  aarch64-apple-darwin      macOS Apple Silicon"
    echo "  x86_64-apple-darwin       macOS Intel"
    echo "  x86_64-pc-windows-msvc    Windows 64-bit"
    exit 0
}

TARGETS=()

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
    usage
elif [[ "${1:-}" == "--all" ]]; then
    TARGETS=(aarch64-apple-darwin x86_64-apple-darwin x86_64-pc-windows-msvc)
elif [[ "${1:-}" == "--target" ]]; then
    TARGETS=("${2:?--target requires a triple argument}")
else
    # Auto-detect current platform
    OS="$(uname -s)"
    ARCH="$(uname -m)"
    case "$OS" in
        Darwin)
            if [ "$ARCH" = "arm64" ]; then
                TARGETS=(aarch64-apple-darwin)
            else
                TARGETS=(x86_64-apple-darwin)
            fi
            ;;
        MINGW*|MSYS*|CYGWIN*)
            TARGETS=(x86_64-pc-windows-msvc)
            ;;
        *)
            echo "Unsupported platform: $OS. Use --target to specify manually."
            exit 1
            ;;
    esac
fi

download_macos() {
    local TARGET="$1"
    local TMPFILE=$(mktemp /tmp/ffmpeg-XXXXXX.zip)

    echo "  Downloading ffmpeg for macOS ($TARGET)..."
    curl -L -o "$TMPFILE" "https://evermeet.cx/ffmpeg/getrelease/ffmpeg/zip" 2>/dev/null
    unzip -o "$TMPFILE" -d "$BINARIES_DIR" > /dev/null
    mv "$BINARIES_DIR/ffmpeg" "$BINARIES_DIR/ffmpeg-$TARGET"
    chmod +x "$BINARIES_DIR/ffmpeg-$TARGET"

    # Remove quarantine attribute and ad-hoc sign for macOS Gatekeeper
    xattr -cr "$BINARIES_DIR/ffmpeg-$TARGET" 2>/dev/null || true
    codesign --force --sign - "$BINARIES_DIR/ffmpeg-$TARGET" 2>/dev/null || true

    rm -f "$TMPFILE"
    echo "  ✓ Saved and signed: ffmpeg-$TARGET"
}

download_windows() {
    local TARGET="$1"
    local TMPFILE=$(mktemp /tmp/ffmpeg-XXXXXX.zip)

    echo "  Downloading ffmpeg for Windows ($TARGET)..."
    curl -L -o "$TMPFILE" "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip" 2>/dev/null
    unzip -o "$TMPFILE" -d "$BINARIES_DIR" > /dev/null

    # Find ffmpeg.exe in the extracted directory
    local FFMPEG_EXE=$(find "$BINARIES_DIR" -name "ffmpeg.exe" -path "*/bin/*" | head -1)
    if [[ -z "$FFMPEG_EXE" ]]; then
        FFMPEG_EXE=$(find "$BINARIES_DIR" -name "ffmpeg.exe" | head -1)
    fi

    cp "$FFMPEG_EXE" "$BINARIES_DIR/ffmpeg-$TARGET.exe"

    # Clean up extracted directory
    rm -rf "$BINARIES_DIR/ffmpeg-"*"-essentials_build" 2>/dev/null || true
    rm -rf "$BINARIES_DIR/ffmpeg-"*"-essentials" 2>/dev/null || true
    rm -f "$TMPFILE"
    echo "  ✓ Saved: ffmpeg-$TARGET.exe"
}

echo "Downloading ffmpeg for ${#TARGETS[@]} platform(s)..."

for TARGET in "${TARGETS[@]}"; do
    case "$TARGET" in
        *-apple-darwin)
            download_macos "$TARGET"
            ;;
        *-pc-windows-msvc)
            download_windows "$TARGET"
            ;;
        *)
            echo "  ✗ Unsupported target: $TARGET"
            ;;
    esac
done

echo ""
echo "Done! Binaries in $BINARIES_DIR:"
ls -lh "$BINARIES_DIR"/ffmpeg-* 2>/dev/null
