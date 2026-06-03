#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

usage() {
    echo "Usage: $0 [--target <triple>]"
    echo ""
    echo "Package the built application with sidecar binaries for distribution."
    echo ""
    echo "Options:"
    echo "  --target <triple>  Package for specific target (default: current platform)"
    echo ""
    echo "Examples:"
    echo "  $0                                    # Package for current platform"
    echo "  $0 --target x86_64-pc-windows-msvc   # Package Windows build as zip"
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

# Detect current platform if no target specified
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
BINARIES_DIR="$PROJECT_DIR/src-tauri/binaries"

# Determine exe name and sidecar name
case "$TARGET" in
    *-pc-windows-msvc)
        APP_NAME="frog-voice.exe"
        SIDECAR_NAME="ffmpeg-$TARGET.exe"
        ARCHIVE_NAME="frog-voice-windows-x64"
        ;;
    *-apple-darwin)
        APP_NAME="frog-voice"
        SIDECAR_NAME="ffmpeg-$TARGET"
        ARCHIVE_NAME="frog-voice-macos-${TARGET%%-*}"
        ;;
    *)
        APP_NAME="frog-voice"
        SIDECAR_NAME="ffmpeg-$TARGET"
        ARCHIVE_NAME="frog-voice-$TARGET"
        ;;
esac

# Check exe exists
if [[ ! -f "$RELEASE_DIR/$APP_NAME" ]]; then
    echo "❌ Application not found: $RELEASE_DIR/$APP_NAME"
    echo "   Run: scripts/build.sh --target $TARGET"
    exit 1
fi

# Check sidecar exists
if [[ ! -f "$BINARIES_DIR/$SIDECAR_NAME" ]]; then
    echo "❌ Sidecar not found: $BINARIES_DIR/$SIDECAR_NAME"
    echo "   Run: scripts/download-ffmpeg.sh --all"
    exit 1
fi

# Create package directory
PACKAGE_DIR="$PROJECT_DIR/dist/$ARCHIVE_NAME"
rm -rf "$PACKAGE_DIR"
mkdir -p "$PACKAGE_DIR"

# Copy application binary
cp "$RELEASE_DIR/$APP_NAME" "$PACKAGE_DIR/"

# Copy sidecar binary
cp "$BINARIES_DIR/$SIDECAR_NAME" "$PACKAGE_DIR/"

# For Windows, rename sidecar to just ffmpeg.exe (Tauri expects this naming)
# Actually Tauri uses the target-suffixed name, so keep it as-is.

echo "📦 Packaging $ARCHIVE_NAME..."
echo "   App: $APP_NAME"
echo "   Sidecar: $SIDECAR_NAME"

# Create archive
cd "$PROJECT_DIR/dist"

case "$TARGET" in
    *-pc-windows-msvc)
        # Create zip for Windows
        if command -v zip &>/dev/null; then
            zip -r "$ARCHIVE_NAME.zip" "$ARCHIVE_NAME/" > /dev/null
            echo ""
            echo "✅ Created: dist/$ARCHIVE_NAME.zip"
        else
            echo "⚠️  zip not found, skipping archive. Package directory ready at dist/$ARCHIVE_NAME/"
        fi
        ;;
    *-apple-darwin)
        # Create zip for macOS
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
echo ""
echo "📝 Distribute the archive. Users extract and run $APP_NAME"
