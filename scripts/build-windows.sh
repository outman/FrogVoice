#!/usr/bin/env bash
set -euo pipefail

# Build FrogVoice for Windows (cross-compile from macOS)
#
# Prerequisites (run once):
#   brew install llvm
#   cargo install xwin
#   xwin --accept-license splat --output .xwin
#   rustup target add x86_64-pc-windows-msvc --toolchain nightly

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

TARGET="x86_64-pc-windows-msvc"

echo "🐸 FrogVoice — Windows Cross-Compile Build"
echo ""

# --- Check prerequisites ---

LLVM_BIN="/opt/homebrew/opt/llvm/bin"
if [[ ! -d "$LLVM_BIN" ]]; then
    echo "❌ LLVM not found."
    echo "   Run: brew install llvm"
    exit 1
fi

if [[ ! -d "$PROJECT_DIR/.xwin/crt" ]]; then
    echo "❌ Windows SDK not found."
    echo "   Run: xwin --accept-license splat --output .xwin"
    exit 1
fi

if ! rustup target list --installed --toolchain nightly 2>/dev/null | grep -q "$TARGET"; then
    echo "⚙️  Installing Rust target $TARGET..."
    rustup target add "$TARGET" --toolchain nightly
fi

echo "✅ Prerequisites OK"
echo ""

# --- Build ---

cd "$PROJECT_DIR"

echo "🚀 Building for Windows ($TARGET)..."
echo ""

RUSTUP_TOOLCHAIN=nightly \
PATH="$LLVM_BIN:$PATH" \
pnpm tauri build --target "$TARGET" --no-bundle

# --- Report ---

EXE_PATH="$PROJECT_DIR/src-tauri/target/$TARGET/release/frog-voice.exe"

echo ""
if [[ -f "$EXE_PATH" ]]; then
    EXE_SIZE=$(du -h "$EXE_PATH" | cut -f1)
    echo "✅ Build complete!"
    echo "   Output: $EXE_PATH ($EXE_SIZE)"
else
    echo "⚠️  Build finished but exe not found at expected path"
fi
