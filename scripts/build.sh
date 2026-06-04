#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

usage() {
    echo "Usage: $0 [--target <triple>] [--bundle]"
    echo ""
    echo "Options:"
    echo "  --target <triple>  Build target (default: current platform)"
    echo "  --bundle           Also create installer bundle (requires target platform tools)"
    echo ""
    echo "Examples:"
    echo "  $0                                    # Build for current platform"
    echo "  $0 --target x86_64-pc-windows-msvc   # Cross-compile for Windows from macOS"
    echo "  $0 --target aarch64-apple-darwin      # Build for macOS Apple Silicon"
    echo ""
    echo "Windows cross-compilation from macOS requires:"
    echo "  - rustup target add x86_64-pc-windows-msvc"
    echo "  - rustup toolchain install nightly"
    echo "  - brew install llvm"
    echo "  - xwin --accept-license splat --output .xwin"
    exit 0
}

TARGET=""
BUNDLE=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --target) TARGET="$2"; shift 2 ;;
        --bundle) BUNDLE=true; shift ;;
        --help|-h) usage ;;
        *) echo "Unknown option: $1"; usage ;;
    esac
done

BUILD_ARGS=()
ENV_PREFIX=""

if [[ -n "$TARGET" ]]; then
    BUILD_ARGS+=("--target" "$TARGET")

    # Windows cross-compilation from macOS
    if [[ "$TARGET" == *"windows"* ]] && [[ "$(uname -s)" == "Darwin" ]]; then
        echo "🔀 Cross-compiling for Windows from macOS..."

        # Check prerequisites
        if ! rustup target list --installed | grep -q "$TARGET"; then
            echo "Installing Rust target $TARGET..."
            rustup target add "$TARGET" --toolchain nightly
        fi

        LLVM_BIN="/opt/homebrew/opt/llvm/bin"
        if [[ ! -d "$LLVM_BIN" ]]; then
            echo "❌ LLVM not found. Run: brew install llvm"
            exit 1
        fi

        if [[ ! -d "$PROJECT_DIR/.xwin/crt" ]]; then
            echo "❌ Windows SDK not found. Run: xwin --accept-license splat --output .xwin"
            exit 1
        fi

        # Set up cross-compilation for C code (used by mp3lame-sys via the cc crate)
        TARGET_UNDERSCORE="${TARGET//-/_}"
        XWIN_INCLUDES="-I$PROJECT_DIR/.xwin/crt/include -I$PROJECT_DIR/.xwin/sdk/include/ucrt -I$PROJECT_DIR/.xwin/sdk/include/um -I$PROJECT_DIR/.xwin/sdk/include/shared"
        ENV_PREFIX="RUSTUP_TOOLCHAIN=nightly PATH=\"$LLVM_BIN:\$PATH\" CC_${TARGET_UNDERSCORE}=\"$LLVM_BIN/clang\" CFLAGS_${TARGET_UNDERSCORE}=\"--target=$TARGET $XWIN_INCLUDES\" AR_${TARGET_UNDERSCORE}=\"$LLVM_BIN/llvm-ar\""

        if [[ "$BUNDLE" == false ]]; then
            BUILD_ARGS+=("--no-bundle")
            echo "  ⚠️  Skipping installer bundle (requires makensis on Windows)"
            echo "  📦 Building exe only"
        fi
    fi
fi

echo ""
echo "🚀 Building FrogVoice..."
echo "   Target: ${TARGET:-current platform}"
echo "   Bundle: $BUNDLE"
echo ""

cd "$PROJECT_DIR"

# shellcheck disable=SC2086
eval $ENV_PREFIX pnpm tauri build ${BUILD_ARGS[*]}

echo ""
echo "✅ Build complete!"
if [[ "$TARGET" == *"windows"* ]]; then
    echo "   Output: src-tauri/target/$TARGET/release/frog-voice.exe"
else
    echo "   Output: src-tauri/target/${TARGET:-debug}/release/bundle/"
fi
