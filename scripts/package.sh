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
