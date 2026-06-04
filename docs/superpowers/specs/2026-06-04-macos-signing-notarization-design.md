# macOS Code Signing & Notarization in CI

## Problem

GitHub CI produces unsigned, unnotarized DMG files. macOS Gatekeeper blocks them with "file is damaged" error on Apple Silicon.

## Solution

Add code signing and notarization steps to the macOS build matrix in `.github/workflows/release.yml`. Use manual step management (not tauri-action) for full control and debuggability.

## Architecture

Only macOS builds are affected. Windows builds remain unchanged.

```
Checkout → Rust → Cache → Node → pnpm install
  → [macOS] Import certificate to temp Keychain
  → pnpm tauri build (auto-detects signing identity via APPLE_SIGNING_IDENTITY)
  → [macOS] Notarize DMG via notarytool submit --wait
  → [macOS] Staple notarization ticket via stapler staple
  → Upload release assets
```

## GitHub Secrets Required

| Secret | Description | How to obtain |
|--------|-------------|---------------|
| `MACOS_CERTIFICATE` | Base64-encoded .p12 certificate | `base64 -i cert.p12 \| pbcopy` |
| `MACOS_CERTIFICATE_PASSWORD` | Password set when exporting .p12 | User's own password |
| `APPLE_ID` | Apple ID email | Login email |
| `APPLE_PASSWORD` | App-specific password | Generate at appleid.apple.com |
| `APPLE_TEAM_ID` | 10-char Team ID | Developer Portal top-right |

## Implementation Details

### 1. Temp Keychain (macOS only)

- Create a temporary Keychain with random password
- Set it as default for the session
- Import the .p12 certificate into it
- After build completes, delete the Keychain

### 2. Signing

- Detect signing identity via `security find-identity -p codesigning -v`
- Set `APPLE_SIGNING_IDENTITY` env var so Tauri picks it up automatically
- `pnpm tauri build` will sign the app bundle and DMG

### 3. Notarization

- Submit DMG via `xcrun notarytool submit <dmg> --apple-id --password --team-id --wait`
- `--wait` blocks until Apple finishes (typically 1-5 min)
- On failure, `xcrun notarytool log` outputs the error details

### 4. Stapling

- `xcrun stapler staple <dmg>` attaches the notarization ticket
- This lets Gatekeeper verify offline without contacting Apple servers

### 5. Conditional execution

All signing/notarization steps use `if: matrix.platform == 'macos-latest'` so Windows builds are unaffected.

## Files Changed

- `.github/workflows/release.yml` — add signing, notarization, stapling steps
